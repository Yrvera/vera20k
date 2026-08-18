# MapClass — Ghidra Research Follow-Up (2026-04-22)

> **⚠ PARTIALLY SUPERSEDED 2026-04-24.** This follow-up correctly
> identified `Init_Clear` at `0x5659F0` (§4) and many helpers, but its
> **vtable enumeration is wrong** — it treated 64 dwords as MapClass
> slots when the actual vtable is 30 slots. Addresses past `0x7ED47C`
> belong to the adjacent `VectorClass<CellClass*>` vtable (referenced
> by the embedded field at MapClass+0x138), not MapClass.
>
> For the current vtable + struct status, trust:
> - [`MAPCLASS_COMPLETE_DECODE.md`](MAPCLASS_COMPLETE_DECODE.md) — master summary
> - [`MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md`](MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md) — vtable/struct corrections
>
> **Specific known errors still present in the body below:**
> - §3 "Vtable fully enumerated (64 slots)" — it's 30 slots. Slot 30+
>   rows in the §3 table are unrelated `.rdata` data.
> - §3 slot 3 labeled "CellHasBuilding-style flag check" — correct
>   name is `IsCellExplored` (returns `(cell.ShroudFlags >> 3) & 1`).

Extends `MAPCLASS_GHIDRA_REPORT.md` with newly decompiled functions, vtable
enumeration, and answers to previously-open struct-layout questions.

**Confidence:** HIGH for newly named functions (all decompiled and cross-checked
against callers); MEDIUM for vtable slot purposes inferred from position alone.

---

## 1. What changed vs the original report

- **Vtable enumerated** (originally claimed "64 slots at 0x7ED404"; corrected 2026-04-24 to **30 slots** — see §3 banner).
- **Four previously-open offsets resolved** (+0x134, +0x1158, and the zone-graph
  node structure at +0x24).
- **13 new functions decompiled** covering: map reset, view/tactical coord
  transforms, full-map attribute recalc, overlay flood-fill, fog-of-war border
  update, bridge shroud recalc, bridge ramp state machines, bridge-aware path
  resolution, bridge zone graph edge population, neighbor-bridge redraw.
- **Init_Clear formally identified** (was `FUN_005659F0`) via leaked debug
  string `s_MapClass__Init_Clear_entry_0082acc4`.
- **Two zone-system incremental helpers explained** (AssignOrphanedCellZone /
  MergeAdjacentCellZone — the cheap alternative to full `UpdateBridgeZonesHelper`).

The original report's struct layout, bridge record format, crate slot table,
and zone speed cache documentation are still correct — this is additive.

---

## 2. Resolved open questions

### ~~+0x134~~ → **cells-destroyed counter / scenario re-init flag**

`FUN_00565BC0` (cell-array destructor walk) writes `*(this + 0x134) = 0` at
entry before destroying all 0x40000 CellClass instances. Combined with
`ScenarioClass::Full_Init` (0x687B9C) also writing this field, +0x134 tracks
scenario load/unload state — zeroed when the cell grid is torn down, set during
full init. Safe to treat as scalar state; not used for gameplay logic.

### +0x1158 → **1-byte init flag** (newly discovered)

`MapClass::Init_Clear` (FUN_005659F0) writes `*(this + 0x1158) = 0` alongside
`*(this + 0x148) = 0xD` (num_movement_zones). Sits in the 4 bytes of padding
between the crate slot table (ends at +0x1157) and the final DynamicVectorClass
(starts at +0x115C). Purpose: some scenario-reset flag — no readers found in
the functions decompiled so far. Not previously documented.

### Zone-graph node structure at +0x24 (was Open Question #6)

`MapClass__AddBridgeZoneEdges` (0x5851B0) reads/writes these nodes directly:

```
ZoneGraphNode — 36 bytes (0x24), indexed as zone_conn_vec[0].data[zone_id * 0x24]
  +0x00: vtable ptr        — polymorphic "can I push another edge?" check
  +0x04: edges_ptr         — int[2]-edge array: { target_zone_id, weight }
  +0x08: capacity          — max edges before grow
  +0x0C: owns_memory (char)
  +0x0D: is_valid (char)
  +0x10: count             — current edge count
  +0x14: grow_step         — = 0x14 (20) per original report
  +0x18: cost_weight (ushort)
  +0x1C: cost_type (int)   — indexes cost table at 0x7E3794
  +0x20: (padding or flags)
```

Push-back pattern: if `count < capacity`, store `{target, weight}` at
`edges_ptr[count * 8]`; else call vtable+8 to grow.

### Vtable ~~(5 slots documented)~~ → full 64-slot enumeration

See §3 below. Most slots 31–63 are inherited GScreenClass input handlers
(Clicked/RightClicked/MouseHover/focus/etc.) implemented as FUN_0058xxxx.

> **Correction (2026-04-24):** The "64 slots" claim here is wrong —
> see §3 for the full correction banner. MapClass vtable has **30
> slots**. What this follow-up interpreted as inherited input
> handlers in slots 31–63 are actually DisplayClass's handlers living
> in DisplayClass's own vtable at `0x7E6114`.

### Remaining still-open

- **+0x74–0x7F (12 bytes):** still no reads found. Likely genuinely unused
  scalar slots in YR — candidate for "reserved" in a Rust mirror.
- **+0x11C–0x123 (8 bytes):** still no reads. Same status.

---

## 3. MapClass vtable (0x7ED404, 64 slots)

Decoded from raw memory at 0x7ED404, 256 bytes. Slots identified via function
labels and inferred where unlabeled.

| Slot | Address | Inferred purpose |
|------|---------|------------------|
| 0 | 0x4F4240 | scalar deleting destructor (GScreenClass base) |
| 1 | 0x40D230 | inherited ctor-helper |
| 2 | 0x40D240 | inherited ctor-helper |
| 3 | 0x5656D0 | **`IsCellExplored`** — reads `(cell.ShroudFlags >> 3) & 1` (corrected 2026-04-24; was "CellHasBuilding-style") |
| 4 | 0x588BF0 | scalar-deleting dtor (MapClass override) |
| 5 | 0x565800 | **Init / Alloc** — allocates cell array, zone tables |
| 6 | 0x4F42B0 | inherited |
| 7 | 0x5659F0 | **MapClass::Init_Clear** — reset crate timers, flags |
| 8–13 | 0x4F42E0..0x4F4450 | GScreenClass base virtuals |
| 14 | 0x4F42F0 | MarkNeedsRedraw(2) |
| 15 | 0x4F4480 | inherited |
| 16 | 0x4AEBD0 | inherited |
| 17 | 0x4F45B0 | inherited |
| 18–21 | 0x4C9150 ×4 | abstract placeholders (same addr for 4 slots) |
| 22 | 0x565AA0 | reset CellClass* VectorClass (null all 262144 slots) |
| 23 | 0x565B00 | cell array resize helper |
| 24 | 0x565BC0 | **destroy all CellClass instances** (zeroes +0x134 at entry) |
| 25 | 0x577920 | UnregisterBridgeRepairHut |
| 26 | 0x4AEBE0 | inherited |
| 27 | 0x56BBE0 | **UpdateCrateRegenTimers** (per-tick) |
| 28 | 0x565C10 | **map cell init** (resize + Size/LocalSize parsing) |
| 29 | 0x567230 | **viewport-resize handler** (updates Techno on-screen flags, idle voice) |
| ~~30~~ | ~~0x8055E0~~ | ~~thunk/import (outside .text)~~ |
| ~~31–63~~ | ~~0x588xxx..0x58A5B0~~ | ~~GScreenClass input handlers~~ |

> **Correction (2026-04-24):** Slots 30–63 above are NOT MapClass
> vtable entries. The real MapClass vtable ends at slot 29 (`0x7ED47C`
> exclusive). The dwords starting at `0x7ED480` belong to the adjacent
> `VectorClass<CellClass*>` vtable referenced by the embedded field at
> MapClass+0x138. The "input handlers" listed above are inherited by
> *DisplayClass* (a subclass), not MapClass, and they live in
> DisplayClass's own vtable at `0x7E6114`. See
> `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §1 for the proof.

---

## 4. Newly documented functions

### MapClass::Init_Clear — 0x5659F0

Debug string `s_MapClass__Init_Clear_entry_0082acc4` confirms the name.

```
1. Register_heap_pool + Network_ServiceLoop entry
2. ZBuffer_rect_clear(0xFFFF)        — clear z-buffer
3. CircBuf__FillAll(0x7F)            — fill screen buffer with shroud color
4. FUN_004F42D0()                    — base-class clear (GScreenClass)
5. For each of 256 crate slots at +0x158:
     pause the slot timer — stash (duration - elapsed) back into duration,
     write start_frame = -1
6. Set +0x148 (num_movement_zones) = 13
7. Set +0x1158 = 0                   — newly discovered flag
8. Register_heap_pool on exit
```

Called on scenario reset / save-load. Preserves crate regen time remaining by
converting active timers into paused-duration form.

### MapClass::Get_Has_Cell (tentative) — 0x5657E0

```c
bool(this, CellStruct *coord) {
    int idx = coord->Y * 0x200 + coord->X;
    return cell_array[idx] != nullptr;   // no bounds check!
}
```

Fast "does this cell exist" check. **No bounds check** — caller must validate.

### MapClass::Screen_To_Cell / Cell_To_Screen variants — 0x5654A0, 0x565520, 0x565660

Three isometric coordinate transforms, all using MapClass fields:

```
FUN_005654A0 (world+client → cell):
    out.X = ((in.Y + map.local_top + 1) >> 1) + (in.X + map.local_left)
    out.Y = (map.size_width + (in.Y + map.local_top) >> 1) - (in.X + map.local_left)

FUN_00565520 (cell → client, width-parity aware):
    parity = map.size_width & 1
    out.X = ((in.X - in.Y + parity) >> 1 + map.size_width/2) - map.local_left
    out.Y = (in.Y - map.size_width) + in.X - map.local_top

FUN_00565660 (cell → client, short output):
    same formula as 565520 but packs result as CONCAT22(sVar1, sVar2)
```

These are the **camera/cursor math** used by every on-screen rendering path
and mouse-hit test. Critical for tactical view fidelity.

### MapClass::Invalidate_Radius_For_Redraw (tentative) — 0x568140

```
For each cell in a square -r-2..+r+2 around center:
    if (dx² + dy² < (r+1)²):
        cell.render_cache[+0x130] = 0
        cell.render_cache[+0x134] = 0
        cell.flags[+0x12C] |= 0x18     // bits 3 + 4
```

Targeted dirty-rect invalidation — used for explosion VFX, muzzle flashes,
and other bounded redraws. Bit 3 = "needs redraw"; bit 4 = "redraw includes
overlay" (inferred from CellClass flag layout).

### MapClass::Is_Coord_In_Playfield_Inverted — 0x568350

```c
bool(this, CellStruct *coord) {
    int x = coord.X >> 8;        // lepton → cell
    int y = coord.Y >> 8;
    int w = map.size_width;
    return (x+y > w)
        && (x-y < w)
        && (y-x < w)
        && (x+y <= w + map.size_height*2);
}
```

Diamond-bounds check. Same geometry as `Is_Cell_In_Playfield` but takes lepton
coords (not cell coords) as input.

### MapClass::Viewport_Resized — 0x567230

Called when tactical clip rect changes. Does three things:

1. Clips `+0xFC..+0x108` (LocalSize) against AlphaShape clip rect
2. Enforces minimum: LocalSize.left/top ≥ 2; clamps bottom/right inward
3. Iterates **every TechnoClass**, updates their `in_playfield` byte (+0x3D5)
   via `MapClass::Is_Cell_In_Playfield`, and when a unit **newly enters
   playfield** (was 0, now 1), plays the unit's idle voice via vtable slot
   0x120 — but only for human-player units, non-building, non-idle-suppressed.

**Implication:** idle voice playback is driven by scroll/camera movement, not
a timer. Units outside the tactical view don't trigger voice.

### MapClass::InitCellAttributes — 0x568BB0

**Full-map post-load attribute pass.** Two CellIterator sweeps:

**Pass 1 — clear tag-rect flags:**
```
For each cell:
    cell.flags[+0x140] &= ~0x300000    // clear bits 20+21
```

**Pass 2 — per-cell init:**
```
For each cell:
    cell.field_30 = 0
    FUN_00483e30(0, 0x10000, 0, 1000, 1000, 1000)   // ambient sound/VFX init
    cell.Flags &= ~0x20000                           // clear "visited" bit
    if (cell.AttachedTag):
        if (FUN_006E5320())  /* horizontal tag */:
            for x = 0..playfield_width:
                cell_at(cell.Y, playfield_left + x).flags[+0x140] |= 0x100000
        elif (FUN_006E5300())  /* vertical tag */:
            for y = 0..playfield_height:
                cell_at(playfield_top + y, cell.X).flags[+0x140] |= 0x200000
    total_tiberium += (param_2 ? FUN_004818e0(0) : CellClass::Get_Tiberium_Value())
    CellClass::RecalcAttributes(cell)
    if (cell.OverlayTypeIndex >= 0 && overlay[cell.Overlay + 0x2A8]):
        FUN_0047d210()       // overlay-has-anim side effect
Returns: total tiberium value
```

This is the **map-wide "everything needs recalc"** hook called after INI
reload, rules change, or scenario init. Tag-rect stamping means trigger areas
are materialized into cell flag bits once, not scanned every tick.

### MapClass::SetOverlayAndPropagate — 0x56EB80

**Recursive tile-flood-fill.** Replaces `cell.IsoTileTypeIndex` and propagates
to all 8-connected neighbors with the same old tile index.

```
void SetOverlayAndPropagate(cell_coord, new_tile, old_tile, extra, skip_dirty):
    cell = get_cell(cell_coord)
    if (!skip_dirty):
        dirty_tactical_rect_around(cell, 0x100×0x100)
        if (new_tile == cell.IsoTileTypeIndex) return
    if (cell.IsoTileTypeIndex != new_tile):
        cell.IsoTileTypeIndex = new_tile
        CellClass::RecalcAttributes(cell)
        RadarClass::MarkTerrainDirty(cell_coord)
        for dir in 0..8:
            neighbor = cell_at(cell_coord + DirectionOffsets[dir])
            if (neighbor.IsoTileTypeIndex == old_tile):
                SetOverlayAndPropagate(neighbor, new_tile, old_tile, extra, 1)
```

Used by bridge damage/collapse to replace whole connected tile runs
atomically. `skip_dirty` cuts rendering churn on recursive calls.

### MapClass::UpdateFogBorder — 0x567DA0

Fog-of-war edge tile recalc **in a spiral around a center**:

```
void UpdateFogBorder(center, sight_range_min, sight_range_max, fog_flag):
    sight_range = clamp(sight_range_max, 3, 11)
    entries = DAT_007ED3D0[sight_range] - (min > 0 ? DAT_007ED3CC[min] : 0)
    if (!fog_flag):  // fog DISABLED path
        spiral_base = DAT_00ABD490 + (min > 0 ? DAT_007ED3CC[min]*2 : 0)
        for e in spiral_base..(spiral_base + entries):
            cell = lookup(center + spiral_offset[e])
            new_bitmask = Shroud_EdgeBitmask_Calculator(cell_coord, 0)
            if (new_bitmask != cell.fog_edge_bitmask_0x120):
                cell.fog_edge_bitmask_0x120 = new_bitmask
                cell.dirty_flag_0x138 = 1
                FUN_006DA7D0(cell)  // render-side update
```

Uses the **same spiral tables as RevealShroud** (`InitRevealSpiralTable` at
0x561910). The `param_5 == 0` gate + reversed-logic comment in the original
report's open questions point to this being the **TS-era fog-of-war renderer**
— gated by `SpecialFlags.FogOfWar`. In stock YR this function is never reached
because fog is disabled by default.

### MapClass::RecalcBridgeShroudFlags — 0x578100

Two passes over all cells:

```
Pass 1 — for each cell with flag[+0x140] & 0x20 (bridge-surface):
    cell.flags[+0x12C] &= ~0x18         // clear "dirty" bits
    cell.flags[+0x140] &= ~0x1C         // clear shroud propagate bits
    cell.dirty_0x138 = 1
    FUN_006DA7D0(cell)
    if cell.edge_bitmask_0x120 == -2:   // 0xFE = fully-shrouded marker
        CellChangeNotify(cell, player, 1)

Pass 2 — for every cell:
    coord = (cell.MapCoord + 0x80) >> 8   // lepton → cell
    new_bitmask = Shroud_EdgeBitmask_Calculator(coord, 0)
    if (new_bitmask != cell.edge_bitmask_0x120):
        cell.edge_bitmask_0x120 = new_bitmask
        cell.dirty_0x138 = 1
        FUN_006DA7D0(cell)
        if (new_bitmask == -2):
            CellChangeNotify(cell, player, 1)
```

Called after bridge state changes. Ensures that shrouded cells under a
now-destroyed bridge pick up the correct edge tiles.

### MapClass::UpdateRamp_NS_CollapseA_Low — 0x56EF50 *(representative of the 12-function UpdateRamp family)*

Bridge ramp state machine. The 12 functions cover `{EW|NS}×{High|Low}×{CollapseA|CollapseB|DamageA|DamageB}`.

```
void UpdateRamp_NS_CollapseA_Low(cell_coord, direction):
    neighbor = cell_at(cell_coord + DirectionOffsets[direction & 7])
    if (neighbor.flags[+0x140] & 0x80):  // passable ramp marker
        if (neighbor.level_step[+0x11E] < 7):
            neighbor.level_step[+0x11E] = 7       // force damage level
        else if (neighbor.level_step[+0x11E] == 8):
            UpdateRamp_NS_CollapseA_Low(neighbor, direction)   // recurse
            CellClass::SetBridgeDirection_NWSE(0, 0)
            neighbor.level_step[+0x11E] = 0
            neighbor.overlay_anim_ptr[+0x44] = -1
            RadarClass::MarkTerrainDirty(neighbor.map_coord)

    tile_id_offset = neighbor.IsoTileTypeIndex - DAT_00ABAD1C + 1   // low-bridge base tile
    if (tile_id_offset == DAT_00ABC1E8 || == DAT_00AA0E38):
        ToggleBridgePavement(neighbor, 1, 0)
    elif (tile_id_offset ∈ {DAT_00ABAD30, +2}):
        SetOverlayAndPropagate(neighbor, DAT_00ABAD30+2+DAT_00ABAD1C, -1, -1, 0)
    elif (tile_id_offset == DAT_00ABAD30 + 3):
        UpdateRamp_NS_CollapseA_Low(neighbor, direction)   // recurse
        if (!(neighbor.flags[+0x11A] & 1)):
            // NS orientation — blow up 3 cells in column Y direction
            CellClass::BlowUpBridge(cell_at(neighbor.coord))
            CellClass::BlowUpBridge(cell_at(neighbor.coord + {0,-1}))
            CellClass::BlowUpBridge(cell_at(neighbor.coord + {0,+1}))
        else:
            // EW orientation — blow up 3 cells in column X-1 direction
            ... (similar pattern, shifted by {0,-1})
        SetOverlayAndPropagate(neighbor, DAT_00ABAD30+3+DAT_00ABAD1C, -1, cell.level-4, 0)
```

Key constants (from `.data`):
- `DAT_00ABAD1C` = low-bridge tile base index
- `DAT_00ABAD30` = tile offset for collapsed-low segments (+0, +2, +3 are damage variants)
- `DAT_00ABC1E8`, `DAT_00AA0E38` = low-bridge ramp tile offsets
- Cell field `+0x11E` = ramp damage step (0, 7, 8, collapsed)
- Cell field `+0x11A` bit 0 = bridge EW/NS orientation

The 12 functions follow the same pattern with different base constants for
{EW|NS} and {High|Low}. High bridges use `DAT_00AA0E28` as base, low uses
`DAT_00ABAD1C`.

### MapClass::ResolvePathCoord_BridgeAware — 0x583180

Pathfinder helper that resolves "which coordinate should I target on this
bridge cell?" based on path direction.

```
CoordResult ResolvePathCoord_BridgeAware(cell, check_bridge_flag):
    if (!check_bridge_flag || !(cell.Flags & 0x100)):   // 0x100 = is-bridge
        return cell.MapCoord    // regular cell

    bridge_idx = FindBridgeRecord(cell.coord, mode=2, 0)
    bridge = &bridge_record[bridge_idx]

    if (cell.Flags & 0x800):   // EW orientation
        delta.X = 0
        delta.Y = cell.Y - bridge.endpoint_a.Y
    else:
        delta.X = cell.X - bridge.endpoint_a.X
        delta.Y = 0

    if (bridge.is_intact):
        // Pick the closer of the two bridge endpoints
        dist_a = sqrt((ep_a + delta - cell)²)
        dist_b = sqrt((ep_b + delta - cell)²)
        return (dist_a < dist_b) ? ep_a + delta : ep_b + delta

    // Intact == false: walk through chained bridge cells
    while (cell.Flags & 0x100):
        cell = Pathfinding_update_continued()

    if (IsBridge(cell) || IsWoodBridge(cell)) && cell.LandType != 3:
        return ep_b + delta
    return ep_a + delta
```

**Implication for parity:** units navigating onto a bridge need this
coordinate-rewrite, otherwise they path to the bridge-surface cell coord
(unreachable via ground) instead of the bridge ramp endpoint.

### MapClass::AssignOrphanedCellZone — 0x56D460

**Incremental zone repair (add).** When a single cell becomes passable
(building removed, bridge repaired, etc.), instead of rebuilding all zones
via `UpdateBridgeZonesHelper` (expensive), it:

```
1. Look up cell's linear zone_cell_data entry at +0x68
2. If cell type == 7 (impassable), abort
3. For each of 8 neighbors in zone_cell_data:
    if neighbor.zone_type == 0 (unassigned):
        check how many distinct zones touch this cell
        if ≤ 3 conflicts: inherit neighbor's cluster_id, done
        else: full rebuild via UpdateBridgeZonesHelper
    else: continue scanning
4. If no neighbors helped: full rebuild
```

### MapClass::MergeAdjacentCellZone — 0x56D5A0

**Incremental zone repair (merge).** Mirror function — when a cell's type
changes and it now shares type with a neighbor:

```
1. Look up cell at zone_cell_data +0x68
2. If cell type == 7, abort
3. For each 8-neighbor:
    if neighbor.type == cell.type:
        check distinct-zones-touching count
        if < 4: adopt neighbor's cluster_id
        else: full rebuild
    else continue
4. Fallback to full rebuild
```

These two functions are the **hot path** for cell mutations. Full rebuild is
only hit on topology changes; single-cell edits stay local.

### MapClass::AddBridgeZoneEdges — 0x5851B0

For a bridge cell, adds the "bridge-spanning" zone adjacency edges into the
zone connection graph for all 3 speed categories.

```
For each of 3 speed categories (0, 1, 2):
    zoneA = zone_speed_cache[CellToZoneIndex(endpoint_a_side) * 10 + speed*2]
    zoneB = zone_speed_cache[CellToZoneIndex(endpoint_b_side) * 10 + speed*2]
    push edge A→B into zone_conn_vec[speed][zoneA]
    push edge B→A into zone_conn_vec[speed][zoneB]
    // repeat for two more adjacent bridge edge pairs (4 total directional pairs)
```

Reads tile-direction lookup at `DAT_0082A944` (indexed by `IsoTileTypeIndex -
bridge_base`) to determine which way the bridge points. Paired with
`RemoveBridgeZoneEdges` (0x584E50, called when the bridge is destroyed).

### MapClass::UpdateAdjacentBridges — 0x571050

Walks 8 neighbors of a given cell looking for bridge cells (flag &
0x500 = bridge-side + bridge-surface). If found, scans along the bridge up to
4 segments in the direction that points away from water, then dispatches to
the right `UpdateBridgeEdgeTiles_Low` / `UpdateBridgeEdgeTiles_High` variant
based on tile class, and dirties the affected tactical rect.

Called when something **next to a bridge** changes (overlay placed, building
constructed/removed) so the bridge re-selects its visual edge tiles.

---

## 5. Integration notes for Rust parity

The Rust engine currently splits MapClass responsibilities across
`src/map/` (static) and `src/sim/` (dynamic). Things to watch when
mirroring:

**Rendering / camera math (newly mapped):**
- `FUN_005654A0`, `FUN_00565520`, `FUN_00565660` are the **world↔cell
  coordinate transforms** used by every click and draw path. If the Rust
  camera math drifts from this formula even by a pixel, cursor hit-tests
  will misalign. Worth auditing `src/render/tactical/` and
  `src/sim/geometry/` against these exact expressions.

**Viewport-driven idle voice (slot 29):**
- `MapClass::Viewport_Resized` ties idle voice playback to
  `in_playfield` transitions. This is a 99%-parity item — voice lines
  "waking up" when you scroll onto a unit is a signature feel of RA2.
  Not currently implemented in Rust (no `in_playfield` byte on Techno).

**Bridge system (reinforced understanding):**
- 12 `UpdateRamp_*` functions form a **bridge ramp state machine** keyed
  on cell field `+0x11E` (damage step: 0→7→8→collapse). Rust
  `src/sim/bridge_state.rs` needs to reflect this multi-step collapse
  animation, not just a binary intact/destroyed flag.
- `ResolvePathCoord_BridgeAware` is required for any unit pathing across
  a bridge. Missing this produces "unit walks to wrong cell and
  pathfinding fails" bugs on bridge transitions.

**Incremental zone updates:**
- `AssignOrphanedCellZone` and `MergeAdjacentCellZone` are the **fast
  path** for single-cell zone changes. Current Rust
  `src/sim/pathfinding/zone_map.rs` should be checked — if it only
  exposes full-rebuild, large maps will hitch on every building demolish.

**Fog-of-war border (UpdateFogBorder):**
- Confirmed TS-legacy. In YR `[SpecialFlags].FogOfWar` defaults to
  `false`, so this function and its spiral table are never called.
  **Do not port** unless the engine explicitly opts into fog mode.

---

## 6. Remaining gaps worth filling

1. **+0x74–+0x7F (12 bytes):** still unread anywhere. Could confirm by
   running `find_undocumented_by_string` against potential names
   ("ZoneMetadata", "ZoneStats") — low priority.
2. **+0x11C–+0x123 (8 bytes):** same status.
3. **The 12 UpdateRamp_* functions:** 1 documented here, 11 to go. They
   follow a strict pattern; could be tabulated in one pass by a dedicated
   bridge-damage deep-dive (the interesting variance is in the
   base-tile-constants and direction vectors).
4. **Remaining FUN_0056xxxx:** `FUN_00560BF0`, `FUN_005617E0`,
   `FUN_00561180`, `FUN_005602C0` — these are in the MapClass-adjacent
   range but likely belong to reveal-spiral or zone flood-fill internals.
   Not yet traced.
5. **Vtable slots 30, 38, 46, 54, 62 (0x805xxx):** point outside normal
   `.text`. Probably .idata import thunks — confirm by checking memory
   segment at those addresses.
6. **DisplayClass takeover:** MapClass ends at +0x1174 where DisplayClass
   begins. DisplayClass is where most *visible* behavior lives
   (tactical rendering, cell-under-cursor, drag-rect, waypoint plotting).
   **That's the next logical report** — MapClass is now ~95% complete;
   DisplayClass is ~0% documented and is the direct owner of everything
   the player sees and clicks on.

---

## Sources

### Newly decompiled functions (13)

- 0x5654A0, 0x565520, 0x565660, 0x5656D0, 0x5657E0 — coord transforms + flag check
- 0x5659F0 — `MapClass::Init_Clear` (string-confirmed)
- 0x565AA0, 0x565BC0 — cell array reset/destroy
- 0x567230 — viewport resize handler
- 0x567DA0 — `MapClass::UpdateFogBorder`
- 0x568140 — radius-invalidate
- 0x568350 — diamond bounds check (lepton-coord variant)
- 0x568BB0 — `MapClass::InitCellAttributes`
- 0x568E40, 0x569760 — bridge overlay walkers (low/high)
- 0x56EB80 — `MapClass::SetOverlayAndPropagate`
- 0x56EF50 — `MapClass::UpdateRamp_NS_CollapseA_Low` (representative ramp fn)
- 0x571050 — `MapClass::UpdateAdjacentBridges`
- 0x578100 — `MapClass::RecalcBridgeShroudFlags`
- 0x583180 — `MapClass::ResolvePathCoord_BridgeAware`
- 0x5851B0 — `MapClass::AddBridgeZoneEdges`
- 0x588BF0 — scalar deleting destructor

### Raw memory reads

- 0x7ED404 (256 bytes) — full vtable dump → 64 slots

### Referenced from existing doc

- `MAPCLASS_GHIDRA_REPORT.md` (original) — struct layout, bridge records,
  crate slot layout, zone speed cache, zone system outline
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — for CellClass offsets referenced
  in this follow-up
- `BRIDGE_SYSTEM.md`, `ZONE_PASSABILITY_VERIFIED.md` — for zone system context
