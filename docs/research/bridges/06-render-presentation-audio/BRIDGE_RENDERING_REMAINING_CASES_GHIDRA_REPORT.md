# Bridge Rendering Remaining Cases - Ghidra Research Report

**Date:** 2026-05-16
**Primary scope:** remaining bridge rendering parity risks after the high-bridge body Y-offset fix.
**Primary addresses:** `0x00480110`, `0x0047F6A0`, `0x0047F510`, `0x00480350`, `0x00547230`, `0x006D3D10`, `0x006D3290`, `0x006D3040`, `0x0056E990`, `0x0056EB80`, `0x0057ACF0`, `0x00579620`, `0x0057A430`, `0x00484AB0`.
**Confidence:** HIGH for high-bridge body/shadow/frame/ordering facts, HIGH for the existence and shape of railing lookup tables, MEDIUM for exact runtime railing table values because the static table is zero-filled and theater-loaded at runtime, HIGH for low-bridge mask/selector structure.
**Active in YR:** Yes for the standard per-frame Tactical draw path and bridge damage/repair paths. FoggedObject/RMG bridge paths from older reports remain TS-legacy or non-standard and are not used as normal YR bridge rendering evidence.

> **Current correction (2026-08-14):** exact high-bridge body depth is no longer
> an open case. The explicit `CC_Draw_Shape` `0x10` gate is zero, effective flags
> remain `0x4E00`, and stock format-3 bodies route through the extended selector's
> slot `+0x158` to the strict Z-read/write leaf at `0x004990E0`. Native base is
> `-2 - 15 * (signed level + 4)` and gradient entry 0 steps `-1` per full-canvas
> row. Rust now supplies the equivalent local-row depth plane. Railings, shadow
> RGB565 darkening, low-bridge visuals, and `ZFudgeBridge` remain separate cases.

## Summary verdict

`gamemd.exe` does not render bridges as one terrain object. A visible high bridge is a stack of:

1. TMP deck/base terrain in Tactical terrain step 3.
2. SHP body in Tactical terrain step 5, pass 1.
3. SHP shadow in Tactical terrain step 5, pass 2.
4. SHP railings in Tactical terrain step 7.
5. Objects after the terrain bundle; units are not between the binary's bridge body and railings by top-level pass order, but bridge body/TMP/objects still interact through Z-buffer and object Y/depth rules.

The recent Rust body Y-offset fix is consistent with live binary evidence: `CellClass+0x11E` state bytes `0..=8` receive `-16`; state bytes `9..=17` receive `-31`. Do not change that unless a later visual diff proves the Rust axis-to-state mapping is wrong.

The remaining renderer parity gaps are railings, shadows, low-bridge visual/damage pathing, and exact draw/depth interaction. The largest hard gap is that the Rust railing tables are still all-zero placeholders, so no real railings can emit. The next largest visible gap is that bridge body shadows are built but disabled in the draw pass because a shadow/darken blitter equivalent is missing.

## Verified high bridge body rendering facts

### Frame selection

**Verified fact:** `CellClass__DrawOverlay_Body @ 0x0047F6A0` reads `cell+0x11E` as the frame/state byte. If the byte is exactly `0` or exactly `9`, it adds a 4x4 Latin-square value indexed by `((cell.y & 3) << 2) | (cell.x & 3)`. All other state bytes are drawn directly.

**Evidence:** live decompile of `0x0047F6A0`; raw memory read of `0x0081CC30`.

```text
g_OverlayVarietyLatinSquare =
  0,1,2,3,
  3,2,1,0,
  2,3,0,1,
  1,0,3,2
```

**Confidence:** HIGH.
**Active in YR:** Yes; this is called from `Tactical_layer_smudges -> Cell_ContentRendering`.

### State byte mapping

**Verified fact:** the renderer itself treats the byte ranges mechanically:

| State byte | Body Y offset | Frame behavior |
|---|---:|---|
| `0` | `-16` | add Latin-square 0..3 |
| `1..=8` | `-16` | draw exact frame |
| `9` | `-31` | add Latin-square 0..3 |
| `10..=17` | `-31` | draw exact frame |
| `>=18` | no valid bridge-body frame in this path |

**Evidence:** `CellClass__Get_Draw_Offset @ 0x00480110` and `CellClass__DrawOverlay_Body @ 0x0047F6A0`.

**Tiny detail:** destroyed cells do not become state byte `18`. The damage/collapse paths remove or replace the drawable bridge state; `DrawOverlay_Body` also hard-returns for overlay `0xA7` and `0xB2`.

**Confidence:** HIGH.
**Active in YR:** Yes.

### Body Y offset

**Verified fact:** `CellClass__Get_Draw_Offset @ 0x00480110`:

```text
base_y = overlay_type_offset_y
if cell.flags & 0x80:
    base_y -= 0x10
    if 9 <= cell.state_byte <= 0x11:
        base_y -= 0x0f
elif cell.overlay == 0xef:
    base_y -= 0x0f

out.x = base_x + 0x1e
out.y = base_y + viewport_y + signed(cell.level) * -0x0f + 0x0f
```

So high bridge body offset follows the state byte, not the SHP frame family and not a named axis.

**Evidence:** live decompile of `0x00480110`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Body draw layer and depth

**Verified fact:** body SHPs are drawn by `CC_Draw_Shape` with flags `0x4E00` and non-zero Z parameter. `CC_Draw_Shape @ 0x004AED70` stores the Z parameter at frame data offset `+0x17C` and ORs flag `0x10` when the Z parameter is non-zero. `Blitter_selector @ 0x00490B90` then selects a Z-capable blitter for the `0x4E10` body flags.

The Z parameter is:

```text
(signed(cell.level) + ((cell.flags >> 7) & 1) * 4) * -15 - 2
```

The `0x80` bridge flag therefore contributes a +4 height bonus to body SHP depth, but not by changing the screen Y formula directly.

**Evidence:** live decompile of `0x0047F6A0`, `0x004AED70`, `0x00490B90`.
**Confidence:** HIGH for flag/Z selection; previous report already resolved the `0xC0` blitter as Z read/write.
**Active in YR:** Yes.

## Verified railing rendering facts

### Source SHP and table shape

**Verified fact:** railings are emitted by `FUN_00547230 @ 0x00547230`, called only from `FUN_004802A0 @ 0x004802A0`. The draw call uses global `DAT_00ABC554` as the SHP pointer and calls:

```text
CC_Draw_Shape(DAT_00ABC554, table_frame - 1, pos, clip, 0x4601, ..., z_param, ..., 1000, ...)
```

`artmd.ini` declares `[RAILBRDG] Theater=yes`, and `rulesmd.ini` defines `[RAILBRDG1]` / `[RAILBRDG2]` with `Image=RAILBRDG`. The binary evidence identifies the runtime SHP pointer, while the INI/art data identifies the asset family.

**Evidence:** live decompile of `0x00547230`, `0x004802A0`; `ini/artmd.ini` `[RAILBRDG]`; `ini/rulesmd.ini` `[RAILBRDG1]`, `[RAILBRDG2]`.
**Confidence:** HIGH for runtime draw path, MEDIUM for exact asset-name-to-global binding from static-only evidence.
**Active in YR:** Yes.

### Railing lookup tables

**Verified fact:** the table entry stride is 16 bytes:

```text
entry + 0x00: shp_frame_1based
entry + 0x04: required sub_tile
entry + 0x08: x offset
entry + 0x0C: y offset
```

There are two main cases:

1. Bridge tile class in `[DAT_00ABC1F8, DAT_00ABC1F8 + 10)` or `[DAT_00AA1098, DAT_00AA1098 + 10)`: use `DAT_00ABC210 + local_index * 0x10`.
2. Tile class matches one of the fallback ranges rooted at `DAT_00AA102C..DAT_00AA103C`: use `DAT_00ABC2D0 + local_index * 0x10`.

The draw is skipped when:

- the tile class does not match a recognized range;
- `iVar4 - iVar2 == -1`;
- the current cell sub-tile does not equal the table entry's required sub-tile;
- the table frame value is zero.

**Evidence:** live decompile of `0x00547230`; static memory reads of `0x00ABC210` and `0x00ABC2D0` show zero-filled static image, consistent with runtime theater initialization rather than static constants.
**Confidence:** HIGH for entry format and skip conditions; MEDIUM for final table values until captured after theater load.
**Active in YR:** Yes.

### Damaged vs healthy variants

**Verified fact:** the railing draw function does not read `cell+0x11E` directly. It keys off `IsoTileTypeClass+0x294`, the cell sub-tile, and theater-loaded table entries. Damage can affect railings only indirectly when damage/repair code changes the cell's tile index or sub-tile.

**Evidence:** live decompile of `0x00547230`; no read of `cell+0x11E` exists in this function.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Draw order

**Verified fact:** railings are emitted by `Tactical_layer_overlays @ 0x006D3040`, via `FUN_006D7C00 -> FUN_004802A0 -> FUN_00547230`. This is after terrain/TMP and after body/shadow passes, but before the top-level object-rendering bundle in `TacticalClass_Draw @ 0x006D3D10`.

**Evidence:** live decompile of `0x006D3D10`, `0x006D3040`, `0x006D7C00`, `0x004802A0`.
**Confidence:** HIGH.
**Active in YR:** Yes.

## Verified shadow rendering facts

### Body shadow frames and offsets

**Verified fact:** `CellClass__DrawOverlay_Shadow @ 0x0047F510` resolves the same overlay SHP as the body, computes the same base draw offset through `CellClass__Get_Draw_Offset`, then uses:

```text
shadow_frame = shp_frame_count / 2 + state_byte
```

For state bytes `9..=17` on a `flags & 0x80` bridge cell, it applies:

```text
x -= 15
y += 7
```

This is not damage-only. The binary condition is byte range `9..=17`.

**Evidence:** live decompile of `0x0047F510`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Shadow depth and blitter

**Verified fact:** bridge body shadows call `CC_Draw_Shape` with flags `0x4601`, Z parameter `signed(cell.level) * -15 - 2`, and Z height `1000`. There is no +4 bridge-height bonus in the shadow Z parameter.

**Evidence:** live decompile of `0x0047F510`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Railing shadows

**Verified fact:** no separate railing-shadow path was found. Railing uses `0x4601`, the same shadow/darken-style flags used by body shadows, but it draws `DAT_00ABC554` frames from the railing table, not a second shadow frame family.

**Evidence:** live decompile of `0x00547230`.
**Confidence:** MEDIUM. The function proves no separate call in the main railing emitter; asset frame semantics still need visual/table capture.
**Active in YR:** Yes.

## Verified damaged bridgehead TMP variant facts

### High bridgehead TMP variants

**Verified fact:** high bridgehead/ramp visual mutation is event-driven, not per-frame. The renderer consumes the post-mutated tile/overlay fields.

`MapClass__ToggleBridgePavement @ 0x0056E990` toggles `cell.flags & 0x2000`, marks radar dirty, queues a dirty rect, and recursively propagates to 8 neighbors that share the old tile index.

`MapClass__SetOverlayAndPropagate @ 0x0056EB80` writes `cell.IsoTileTypeIndex`, calls `CellClass__RecalcAttributes`, marks radar dirty, queues a dirty rect, and recursively propagates to 8 neighbors sharing the old tile index.

**Evidence:** live decompile of `0x0056E990` and `0x0056EB80`; xrefs show both are called from high and low `UpdateRamp_*` damage/collapse helpers.
**Confidence:** HIGH.
**Active in YR:** Yes when bridge damage/repair mutates bridgehead/ramp visuals.

### DamageA / DamageB progression examples

**Verified fact:** high `UpdateRamp_NS_DamageA_High @ 0x00572230`:

- walks one cell in `param_2 & 7`;
- if target has `flags & 0x80` and state byte `<4`, writes state byte `4`;
- if target state byte is `5`, writes `6`;
- otherwise checks tile-class globals and either toggles pavement or writes one of the `DAT_00ABAD30` tile-class variants.

High `UpdateRamp_NS_DamageB_High @ 0x00572330`:

- if target has `flags & 0x80` and state byte `<4`, writes `5`;
- if target state byte is `4`, writes `6`;
- otherwise advances `DAT_00ABAD30 -> DAT_00ABAD30+1 -> DAT_00ABAD30+2`.

High `UpdateRamp_EW_DamageA_High @ 0x00572B80` mirrors this in the `9..=17` state range:

- if state byte is `9..=12`, writes `0x0E`;
- if state byte is `0x0D`, writes `0x0F`;
- otherwise toggles pavement or writes `DAT_00AA1028` variants.

**Evidence:** live decompile of `0x00572230`, `0x00572330`, `0x00572B80`.
**Confidence:** HIGH for these examples; other `UpdateRamp_*` siblings are structurally equivalent per xrefs and prior state-machine report, but not all are restated here.
**Active in YR:** Yes.

### Repair reset behavior

**Verified fact:** repair/reset is not a render-time decision. Prior verified docs show repair paths call tile/state writers and eventually re-enter `RepairBridgeSegment` / low selectors. In the rendering path, no function recomputes bridgehead variants per frame. `DrawOverlay_Body`, `CellOverlay_TileDraw`, and `FUN_00547230` read the cell state they are handed.

**Evidence:** live per-frame draw decompiles contain no call to high bridgehead selectors; xrefs for low selectors show calls from repair/update paths, not from the draw loops.
**Confidence:** HIGH for "not render-time"; MEDIUM for full repair-state reset details because this pass did not re-decompile every repair walker.
**Active in YR:** Yes.

## Verified z/depth ordering facts

### Top-level order

**Verified fact:** `TacticalClass_Draw @ 0x006D3D10` executes terrain bundle functions in this order:

1. `Tactical_ZBufferDirtyClear`
2. `Tactical_layer_shroud_edges`
3. `Tactical_layer_terrain_shadows` -> TMP terrain draw, including bridge TMP
4. `Tactical_layer_base_terrain` -> FoggedObject walker, dormant in standard YR FogOfWar-off play
5. `Tactical_layer_smudges` -> bridge body pass then shadow pass
6. `Tactical_layer_building_overlays`
7. `Tactical_layer_overlays` -> bridge railings
8. `Tactical_layer_animations`

Only after this bundle does the object bundle run, including `Tactical_ObjectRenderingLoop`.

**Evidence:** live decompile of `0x006D3D10`, `0x006D2DE0`, `0x006D3290`, `0x006D3040`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Bridge TMP deck

**Verified fact:** `CellOverlay_TileDraw @ 0x00480350` always passes `1` as the Z-enable argument to `TMP_TileBlitter`. For multi-subtile tiles, if `FUN_005471F0(sub_tile)` returns true, `param_14` becomes `(cell.flags >> 13) & 1`, so `0x2000` selects the damaged/alternate TMP subtile chain.

**Evidence:** live decompile of `0x00480350`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Unit on bridge vs unit under bridge

**Verified fact:** object drawing happens after the terrain bridge bundle, but object bridge behavior is not only pass order. The unit/object render path has bridge-aware Z fudge:

`FUN_004DAFF0` computes:

```text
base_locomotor_z
+ max(ZFudgeColumn * near_column,
      ZFudgeTunnel * near_tunnel,
      ZFudgeCliff  * near_cliff,
      ZFudgeBridge if near_bridge_else_0)
+ extra_z_adjust
```

The bridge-near predicate at `0x00703B10` returns true only when the object is not already marked on-bridge (`(char)param_1[0x23] == 0`) and the current or selected diagonal neighbor cells carry `flags & 0x100` with orientation-sensitive `0x800` checks. Thus `ZFudgeBridge` is for units under/near a bridge, not for units already on the deck.

**Evidence:** live decompile of `0x004DAFF0` and `0x00703B10`; INI has many stock `ZFudgeBridge=7` entries.
**Confidence:** HIGH for predicate and formula shape.
**Active in YR:** Yes.

### Cliff redraw / terrain redraw interactions

**Verified fact:** bridge railings are still inside the terrain bundle before object rendering in the binary. The Rust renderer currently moves cliff redraw and railings after object merge. This may be a deliberate approximation for other parity goals, but it is not the same top-level order as `gamemd.exe`.

**Evidence:** binary order from `0x006D3D10`; Rust order in `src/app_render/draw_passes.rs`.
**Confidence:** HIGH.
**Active in YR:** Yes.

## Verified low bridge rendering/damage overlay facts

### Low bridges are not high bridges at low height

**Verified fact:** low bridge pathing/facts require `CellClass__IsLowBridgeCell @ 0x00484AB0`:

```text
0 <= cell.tube_index < g_TubeArray.count
and cell.land_type == 10
```

The predicate does not read overlay ID directly.

**Evidence:** live decompile of `0x00484AB0`; prior low-bridge supplement.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Low bridge surface and damage visuals are tile/mask driven

**Verified fact:** low bridge surface maintenance is driven by masks and tile rewrites:

- `MapClass__ComputeBridgeAdjacencyMask_Low @ 0x00579B70` checks all 8 neighbors, requiring neighbor level `current + 4` and rejecting certain theater bridge-surface classes through `FUN_004863D0`.
- `MapClass__ComputeBridgeSurfaceMask @ 0x0057B210` builds a separate surface mask using bridge-surface tests and cached cell data.
- `MapClass__UpdateBridgeTile_Low @ 0x0057A430` may write `cell+0x11A = 0`, `cell+0x38 = DAT_00AA0738`, update tube assignment through `FUN_005A0090`, then recurse over all 8 neighbors.
- `MapClass__SelectBridgeTileVariant_Low @ 0x0057ACF0` and `MapClass__SelectDestroyedBridgeTile_Low @ 0x00579620` pick tile classes from exact bit patterns and PRNG choices, then call `MapClass__ApplyBridgeTile`.

**Evidence:** live decompile of all four addresses above.
**Confidence:** HIGH.
**Active in YR:** Yes for low bridge update/damage/repair paths.

### Destroyed low bridge visuals

**Verified fact:** destroyed low bridge visuals are selected by `MapClass__SelectDestroyedBridgeTile_Low @ 0x00579620`, using the low adjacency mask plus neighbor probes and PRNG. It chooses offsets into the destroyed low-bridge tables `DAT_00ABDDA4` / `DAT_00ABDDA6`, sets `g_UIModeLock` from `DAT_00AA1020 + selected_index`, then calls `MapClass__ApplyBridgeTile`.

**Evidence:** live decompile of `0x00579620`.
**Confidence:** HIGH.
**Active in YR:** Yes.

## Which subsystem controls which behavior

| Behavior | Binary owner | Evidence |
|---|---|---|
| High bridge body frame and body/shadow screen offset | `CellClass` draw methods | `0x00480110`, `0x0047F6A0`, `0x0047F510` |
| TMP deck and damaged-TMP variant bit | `CellOverlay_TileDraw` + bridge state writers | `0x00480350`, `0x0056E990` |
| Railing SHP/table emission | renderer/Tactical overlay layer helper | `0x006D3040`, `0x006D7C00`, `0x004802A0`, `0x00547230` |
| Bridgehead/ramp tile mutation | `MapClass` bridge damage/repair helpers | `0x0056E990`, `0x0056EB80`, `0x00572230`, siblings |
| Low bridge surface/destroyed variant choice | `MapClass` low bridge selectors | `0x0057ACF0`, `0x00579620`, `0x0057A430` |
| Low bridge pathing identity | `CellClass` + `TubeClass` | `0x00484AB0` plus prior tube reports |
| Top-level ordering | `TacticalClass` | `0x006D3D10` |
| Unit under-bridge Z fudge | Techno/object render path | `0x004DAFF0`, `0x00703B10` |

## Current Rust comparison

### Already aligned or mostly aligned

- [bridges.rs](src/app_instances/bridges.rs:34) now has the verified body Y offsets: `0..=8 => -16`, `9..=17 => -31`.
- [bridges.rs](src/app_instances/bridges.rs:65) implements body frame selection with Latin-square jitter only for the base healthy state.
- [bridge_state/mod.rs](src/sim/bridge_state/mod.rs:85) encodes `Axis::NS` into `0..=8` and `Axis::EW` into `9..=17`, matching the report's state-byte-based offset rule.
- [bridge_state/mod.rs](src/sim/bridge_state/mod.rs:468) now has a `BridgeRecordKind` high/low discriminator, which matches the later low-bridge zone-record research direction.

### Confirmed differences

- [bridge_railing_atlas.rs](src/render/bridge_railing_atlas.rs:64) and [bridge_railing_atlas.rs](src/render/bridge_railing_atlas.rs:79) still use all-zero concrete/wood railing tables. Binary skips frame zero, so current Rust emits no real railings.
- [draw_passes.rs](src/app_render/draw_passes.rs:69) still disables bridge body shadows. Binary draws shadows every bridge shadow pass through `0x0047F510`.
- [bridges.rs](src/app_instances/bridges.rs:211) builds shadow instances, but [draw_passes.rs](src/app_render/draw_passes.rs:69) does not submit them.
- [bridges.rs](src/app_instances/bridges.rs:258) shifts shadow by `axis == Axis::EW`; the binary shifts by state byte `9..=17`. This is equivalent only if Rust axis encoding remains `Axis::EW -> 9..=17`; future axis refactors must preserve state-byte semantics.
- [draw_passes.rs](src/app_render/draw_passes.rs:174) draws railings after unit/ground merge and cliff redraw. Binary emits railings in the terrain bundle before object rendering.
- [resolved_terrain.rs](src/map/resolved_terrain.rs:1092) treats low bridge deck level as ground level and [resolved_terrain.rs](src/map/resolved_terrain.rs:1095) marks low bridge overlays as `is_low_bridge`, but binary low-bridge identity also requires a valid tube index and land type `10`.
- [bridge_state/mod.rs](src/sim/bridge_state/mod.rs:8) still states low-bridge overlay damage progression is not fully wired into mutable overlay state.

## Confirmed renderer parity gaps

1. **Railings are absent or incomplete.** Runtime table format is known, but Rust table values are placeholder zeroes.
2. **Bridge body shadows are not rendered.** Binary draws them with `0x4601`; Rust disables the pass because the SHP shadow frames need a darken/shadow blitter equivalent.
3. **Shadow depth is likely wrong when re-enabled.** Binary shadow Z uses ground-level `cell.level * -15 - 2`, not bridge `+4`; Rust currently computes shadow depth with `z + BRIDGE_HEIGHT_BONUS`.
4. **Railing ordering differs.** Binary railings are terrain-bundle work before objects; Rust places them after object/cliff merge.
5. **Railing material classification is suspicious.** Rust maps `"BRIDGEB1"` / `"BRIDGEB2"` to `BridgeKind::Concrete`, but the INI names them Wood Bridge and the binary has separate concrete/wood table roots. This needs runtime table capture before changing code.
6. **Low bridge visuals are over-flattened.** Binary destroyed/healthy low visuals use mask selectors and `ApplyBridgeTile`; Rust still treats much of low bridge rendering as ordinary overlay/TMP state.
7. **Low bridge pathing/render facts are not tied to `TubeClass`.** Binary `IsLowBridgeCell` requires a tube index and land type `10`; Rust broad overlay classification can make a low bridge look bridge-like before it is binary-valid as a low bridge cell.

## Implementation implications

- Keep the high bridge body Y-offset mapping by state byte: `0..=8 => -16`, `9..=17 => -31`.
- Implement real railing table population from theater load or live-captured table values before expecting railings to appear.
- Re-enable bridge shadows only with a shadow/darken blitter equivalent; do not palette-render shadow frames as normal art.
- When re-enabling shadows, use the binary condition `state_byte in 9..=17` for `(-15, +7)` displacement and ground-level shadow depth.
- Treat low bridge rendering as a map/tile mutation problem, not as a simple overlay family. Healthy and destroyed variants come from mask selectors and `ApplyBridgeTile`.
- Audit whether Rust's post-object railing draw is intentional. If parity is the target, binary order says railings belong in the terrain bundle before `Tactical_ObjectRenderingLoop`; if visual testing shows the current order better matches screenshots, document why.

## Remaining open questions

1. **Exact railing table values after theater load.** Static memory at `0x00ABC210` and `0x00ABC2D0` is zero. A live debugger capture after theater initialization is needed for exact `(frame, sub_tile, dx, dy)` values.
2. **Concrete vs wood table mapping for `BRIDGEB1/2`.** The code has distinct table roots; the current Rust naming may be wrong or the table names may not correspond to INI display names. Needs runtime capture.
3. **Final visual order for objects under/on bridge vs railings.** Binary top-level order is clear, but object sprite/Z-buffer internals can still affect overlap. A screenshot diff with a tank on deck and a unit/ship under the same bridge should validate which Rust ordering is visually closest.
4. **Low bridge destroyed-art table to retail TMP names.** The selector indices are known, but the exact theater tile names/indices should be dumped per theater.
5. **Railing shadow semantics.** No separate railing-shadow call was found, but the `0x4601` blitter semantics and RAILBRDG frame art should be visually checked.

## Sources

### Live Ghidra decompile / memory reads

- `0x00480110` `CellClass__Get_Draw_Offset`
- `0x0047F6A0` `CellClass__DrawOverlay_Body`
- `0x0047F510` `CellClass__DrawOverlay_Shadow`
- `0x00480350` `CellOverlay_TileDraw`
- `0x004802A0` railing trampoline
- `0x00547230` bridge railing emitter
- `0x006D3D10` `TacticalClass_Draw`
- `0x006D2DE0` terrain/TMP layer
- `0x006D3290` body/shadow layer
- `0x006D3040` railing layer
- `0x006D6D10` cell body/shadow loop
- `0x006D7C00` railing cell loop
- `0x004AED70` `CC_Draw_Shape`
- `0x00490B90` `Blitter_selector`
- `0x004DAFF0` object Z-fudge formula
- `0x00703B10` bridge-near predicate for ZFudgeBridge
- `0x0056E990` `MapClass__ToggleBridgePavement`
- `0x0056EB80` `MapClass__SetOverlayAndPropagate`
- `0x00572230` `MapClass__UpdateRamp_NS_DamageA_High`
- `0x00572330` `MapClass__UpdateRamp_NS_DamageB_High`
- `0x00572B80` `MapClass__UpdateRamp_EW_DamageA_High`
- `0x0057ACF0` `MapClass__SelectBridgeTileVariant_Low`
- `0x00579620` `MapClass__SelectDestroyedBridgeTile_Low`
- `0x0057A430` `MapClass__UpdateBridgeTile_Low`
- `0x0057B210` `MapClass__ComputeBridgeSurfaceMask`
- `0x00579B70` `MapClass__ComputeBridgeAdjacencyMask_Low`
- `0x0057B440` `MapClass__ApplyBridgeTile`
- `0x00484AB0` `CellClass__IsLowBridgeCell`
- `0x00485060` `CellClass__IsOnBridgeSurface`
- `0x00574600` `MapClass__IsLowBridgeEndpointTile`
- `0x0081CC30` Latin-square raw memory
- `0x00ABC210`, `0x00ABC2D0` railing table static memory

### Existing reports cross-checked

- `docs/research/BRIDGE_BODY_Y_OFFSET_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_RENDERING_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`
- `docs/research/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`
- `docs/research/BRIDGE_SYSTEM.md`
- `docs/gap-scans/2026-05-15-disparity-scan-bridges-end-to-end.md`

### INI and Rust files checked

- `ini/artmd.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/rules.ini`
- `src/app_instances/bridges.rs`
- `src/render/bridge_atlas.rs`
- `src/render/bridge_railing_atlas.rs`
- `src/map/resolved_terrain.rs`
- `src/app_render/draw_passes.rs`
- `src/sim/bridge_state/mod.rs`
- `src/sim/bridge_specs.rs`
- `src/sim/world/bridge_orchestrator.rs`
