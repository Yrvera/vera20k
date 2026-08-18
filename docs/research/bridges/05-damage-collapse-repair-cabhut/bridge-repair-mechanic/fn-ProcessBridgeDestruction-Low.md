# MapClass::ProcessBridgeDestruction_Low — Decode Doc

**Function:** `ProcessBridgeDestruction_Low`
**Address:** `0x00570050`
**Body range:** `0x00570050 – 0x00570AD3`
**Calling convention:** `__thiscall` — `this` (MapClass*) in ECX; `param_2` = cell coord pointer
**Scope:** Full function — bridge repair orchestrator for low (wooden) bridges.

---

## Summary

`ProcessBridgeDestruction_Low` is the primary repair orchestrator for low bridges. Despite the
"Destruction" name, this function implements the **repair** sequence triggered when an engineer
(Infantry with `Engineer=yes`) enters a bridge repair hut (CABHUT). It:

1. Scans a 5×5 area centered on the input coord for any low-bridge overlay cell
   (`CellClass+0x44 ∈ (0x49, 0x66)`).
2. If found → immediately delegates to `MapClass::RepairBridge_Low` and returns.
3. If not found → performs a flag-based fallback walk (same pattern as `DestroyBridge_Low_OnHutDeath`)
   to find the nearest bridge-anchor cell using `CellClass+0x140` flags (0x500 mask → 8-direction
   walk up to 3 steps; then branch by 0x100/0x400/0x80/0x800 flags).
4. With the anchor resolved, enters the tile-type dispatch loop based on `CellClass+0x38`
   tile type vs. tile-set globals.
5. After the dispatch: if `ValidateBridgeZones` returned truthy → `UpdateBridgeZonesHelper`.
6. Tail: if any cells were queued for recalc → `RecalcCellsAndRebuildZones`.

---

## Active in YR

**Yes.** Callers verified via `get_function_callers 0x00570050`:
- `InfantryClass::PerCellProcess @ 0x00519630` — the engineer bridge-repair path.
- `ProcessBridgeDestruction_Low @ 0x00570050` — recursive self-call for multi-span bridges.

Both are live YR paths. Fires on every engineer bridge repair.

---

## Decompilation Behavioral Summary

From `decompile_function 0x00570050`:

```c
// Phase 1: 5x5 overlay scan
for (dy = -2; dy < 3; dy++) {
    for (dx = -2; dx < 3; dx++) {
        cell = Get_CellClass(coord + (dx,dy));
        if (cell->overlay > 0x49 && cell->overlay < 0x66) {
            RepairBridge_Low(coord + (dx,dy));
            return;
        }
    }
}

// Phase 2: flag-based anchor walk (identical to DestroyBridge_Low structure)
cell = Get_CellClass(coord);
if ((cell->bridge_flags & 0x500) == 0) {
    // 8-direction walk, up to 3 steps each direction, looking for 0x500 flag
}
// ... anchor resolution by 0x100/0x400/0x80/0x800 flags

// Phase 3: tile-type dispatch
tile_offset = (cell->tile_type_index - DAT_00ABAD1C) + 1;
// → dispatch to appropriate theater ramp handler (see table below)

// Phase 4: zone rebuild
if (ValidateBridgeZones_result) UpdateBridgeZonesHelper();
if (cells_queued > 0) RecalcCellsAndRebuildZones();
```

Key structural observations:
- `tile_offset = +0x38 - DAT_00ABAD1C + 1` (note the `+1` — distinct from `DestroyBridge_Low_OnHutDeath` which uses `- DAT_00ABAD1C` without `+1`).
- Sub-tile values in this function are the **repair-state** variants (`0x05`, `0x07`), not the destroyed-state variants (`0x04`, `0x02`) checked by `IsBridgeRampTile`.
- `CellClass+0x11B` (adjacent to sub-tile `+0x11A`) is **incremented by 4** on 3 cells during the tile+4 SetOverlay phase — this advances the repair state.
- Recursive self-call targets `coord + (0, -2)` for NS bridges (theater-B tile+4) and `coord + (-2, 0)` for EW bridges (theater-D tile+4).

---

## Callees

Verified via `get_function_callees 0x00570050`:

| Callee | Address | Role |
|---|---|---|
| `MapClass::Get_CellClass` | `0x005657A0` | Cell pointer from coord |
| `MapClass::RepairBridge_Low` | `0x0057F200` | Repair a single low-bridge span (phase 1 fast path) |
| `MapCoord_Add` | `0x0042D510` | Coord arithmetic helper |
| `MapClass::ToggleBridgePavement` | `0x0056E990` | Toggle pavement state of a ramp cell |
| `MapClass::BridgePavementSpanWalker` | `0x00569760` | Walk bridge span to update pavement cells |
| `MapClass::SetOverlayAndPropagate` | `0x0056EB80` | Set overlay type on a cell and propagate |
| `MapClass::ValidateBridgeZones` | `0x0056DB70` | Validate bridge zone connectivity; returns bool |
| `MapClass::UpdateBridgeZonesHelper` | `0x0056C510` | Rebuild bridge zones |
| `MapClass::RecalcCellsAndRebuildZones` | `0x00586990` | Rebuild pathfinding zones for dirty cells |
| `TacticalClass::DirtyScreenRect` | `0x006D2790` | Mark screen rect for redraw |
| `ProcessBridgeDestruction_Low` | `0x00570050` | Recursive self-call for multi-span repair |

---

## Tile-Type Dispatch Table

| Condition | Sub-tile | Tile+4? | Action |
|---|---|---|---|
| `tile_off == DAT_00ABC2B4` | `0x08` | — | TogglePavement → SpanWalker(2) → DirtyRect |
| `tile_off == DAT_00AA1130` | `0x08` | — | TogglePavement → SpanWalker(2) → DirtyRect |
| `tile_off == DAT_00AA1548` | `0x0C` | — | TogglePavement → SpanWalker(4) → DirtyRect |
| `tile_off == DAT_00AA0740` | `0x0C` | — | TogglePavement → SpanWalker(4) → DirtyRect |
| `tile_off ∈ {ABAD30+0..3}` | `0x05` | no | SpanWalker(2) → DirtyRect |
| `tile_off == ABAD30+4` | `0x05` | yes | SetOverlay(ABAD30-1+ABAD1C) + adj+0x11B+=4 × 3 + ValidateZones + **recursive(Y-2)** + SpanWalker(2) |
| `tile_off ∈ {AA1028+0..3}` | `0x07` | no | SpanWalker(4) → DirtyRect |
| `tile_off == AA1028+4` | `0x07` | yes | SetOverlay(AA1028-1+ABAD1C) + adj+0x11B+=4 × 3 + ValidateZones + **recursive(X-2)** + SpanWalker(4) |
| No match | — | — | Continue walk loop (goto phase-2 advance) |

After theater-B or theater-D dispatch (any case): if `ValidateBridgeZones` returned nonzero → `UpdateBridgeZonesHelper`.

---

## CellClass Fields Used

| Field | Offset | Usage |
|---|---|---|
| `overlay_type_index` | `+0x44` | Phase 1: band check `(0x49, 0x66)` |
| `bridge_flags` | `+0x140` | Phase 2: anchor walk (0x80, 0x100, 0x400, 0x500, 0x800) |
| `coord` | `+0x24` | Anchor coord |
| `neighbor_cell` | `+0x2C` | Bridgehead ptr for body cells without 0x80 flag |
| `tile_type_index` | `+0x38` | Phase 3: tile dispatch |
| `sub_tile_index` | `+0x11A` | Phase 3: repair-state sub-tile matching |
| `sub_tile_b` | `+0x11B` | Incremented by 4 on 3 adjacent cells during tile+4 SetOverlay |

---

## Unverified (YELLOW)

- `FUN_0042FCB0` — called with `(0,0)` early in body. Not decoded.
- `DAT_00ABAD1C` — tile-set index base for the `+1` offset arithmetic. Read as zero at static
  time (runtime-populated). Role as index base is inferred; not verified via `read_memory`.
- Sub-tile values `0x05` (theater-B repair) and `0x07` (theater-D repair) are inferred from
  the dispatch table in decompilation — not independently verified as "repair state" vs.
  other states.
- `DAT_0089F690`, `DAT_0089F698`, `g_refinery_unload_adjacent_lookup_dx` — direction lookup
  globals used in the 3-adjacent-cell increment step. Ghidra's labels; semantics inferred.

---

## Self-Proof (exit gate)

### Claim 1: Function is `ProcessBridgeDestruction_Low` at `0x00570050`

`get_function_by_address 0x00570050` → `ProcessBridgeDestruction_Low`, body
`0x00570050 – 0x00570AD3`. **VERIFIED — matches task spec.**

### Claim 2: Two callers — `InfantryClass::PerCellProcess` and recursive self

`get_function_callers 0x00570050` → `InfantryClass__PerCellProcess @ 0x00519630` and
`ProcessBridgeDestruction_Low @ 0x00570050`. Exactly two. **VERIFIED.**

### Claim 3: Calls `RepairBridge_Low @ 0x0057F200` from phase-1 overlay scan; delegates when low-bridge overlay found

`get_function_callees 0x00570050` → `MapClass__RepairBridge_Low @ 0x0057F200` listed.
`decompile_function 0x00570050` → `MapClass__RepairBridge_Low(&param_2)` called when
`*(int *)(iVar3 + 0x44) > 0x49 && *(int *)(iVar3 + 0x44) < 0x66`. **VERIFIED.**
