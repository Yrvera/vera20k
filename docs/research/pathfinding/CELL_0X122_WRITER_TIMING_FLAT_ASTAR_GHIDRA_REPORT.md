# CellClass+0x122 Writer Timing for Flat Hierarchical A*

**Date:** 2026-05-24  
**Mode:** exhaustive-slice  
**Target question:** Which object/blocker writers increment/decrement `CellClass+0x122`, when is the byte read by flat hierarchical A*, and which Rust blocker surfaces can reproduce it?  
**Non-goals:** Do not investigate temp edge buckets, retry producer, layered A*, slope, stock Carville route, or direction-8 tube behavior.  
**Evidence needed to mark COMPLETE:** confirm the A* reader address and polarity; confirm writer source classes and byte RMW pattern; determine whether the field is layer-scoped or global CellClass state; map verified writer semantics to current Rust hard/static/soft blocker surfaces; list any unavailable Rust inputs.  
**Stop conditions:** stop after the writer/read contract and Rust handoff are resolved; record static-object source gaps instead of expanding into a terrain-object subsystem investigation.

## 1. Executive Result

`CellClass+0x122` is a global per-cell byte refcount of blocking/occupying objects in the 8 neighboring cells. It is not a movement-layer field, not a terrain passability byte, and not derived from every unwalkable terrain cell.

For flat hierarchical A*, the reader at `AStar_main_loop @ 0x00429A90`, instruction `0x00429EB1`, checks the **candidate neighbor cell's** `+0x122`. In hierarchical mode only, an off-marker candidate with `+0x122 == 0` is skipped; a nonzero count allows the candidate to continue to the ordinary A*/CanEnter/cost path.

**Rust implication:** production `BlockerNeighborCounts` must be built from the same **source blocker cells** that would have performed `CellClass+0x122` INC/DEC in gamemd: units/aircraft/structures, wall overlays, and terrain objects. Current Rust hard entity blocks plus `LayeredEntityBlockMap` cover buildings and units, but they do **not** by themselves cover static wall/terrain-object blockers. `ResolvedTerrainCell.overlay_blocks || terrain_object_blocks` is the closest current source for those static object writers; do not use all non-walkable terrain.

## 2. Verified Reader

**Finding R1 — A* reads `CellClass+0x122` only as a hierarchical marker-gate exception.**  
Active in YR: **Yes.** Evidence: live Ghidra decompile of `AStar_main_loop @ 0x00429A90` shows the neighbor expansion branch:

```c
if ((char)param_2 != '\0') {
    if ((*(char *)(iVar16 + 0x122) == '\0') && (param_7 != '\0')) goto LAB_0042a1a1;
    goto LAB_00429ec7;
}
```

Assembly context at `0x00429EB1..0x00429EC1` confirms `MOV CL, byte ptr [EBX + 0x122]`, `TEST CL,CL`, `JNZ 0x00429EC7`, then test of the hierarchical flag byte at `[ESP+0x74]`, and `JNZ 0x0042A1A1` to skip. This is a boolean zero/nonzero gate; the numeric count magnitude is not otherwise used by A*.

**Finding R2 — `param_7` is supplied from `AStar_pathfind_search` hierarchical mode.**  
Active in YR: **Yes.** Evidence: `AStar_pathfind_search @ 0x0042C900` calls `AStar_main_loop(param_2,param_3,piVar1,param_5,param_6,param_8)` inside the retry loop. It runs `Zone_precheck(...)` only when `(char)param_8 != 0`, logs `"Hierarchical findpath failure"` on precheck failure, and calls `PathfinderClass__UpdateHierarchicalEdges` after hierarchical A* failure. No Rules/SpecialFlags gate appears around the reader.

## 3. Verified Writers and Timing

All semantic writer sites use the same pattern: compute each of 8 adjacent cells with `MapClass::Get_Cell_By_Coord @ 0x005657A0`, load `byte[cell+0x122]` into `DL`, perform `INC DL` or `DEC DL`, then store the byte back. This is byte RMW, not a recompute.

| Writer site | Owner function | Op | Trigger | Active in YR |
|---|---|---:|---|---|
| `0x005FC762` | `OverlayClass::Mark @ 0x005FC570` | INC | wall overlay placed, gated by `OverlayType+0x2A8` wall flag | Yes; wall overlays are standard map/runtime objects |
| `0x004809DD` | `CellClass::PostDestructionWallCleanup @ 0x00480630` | DEC | wall overlay cleanup/removal | Yes; called by wall/overlay destruction paths |
| `0x00481070` | `CellClass::DestroyOverlay @ 0x00480CB0` | DEC | overlay destroyed | Yes; standard overlay destruction path |
| `0x00440CD9` | `BuildingClass::Unlimbo @ 0x00440580` | INC | building placed, loops footprint/occupy cells | Yes; normal building placement/load |
| `0x00445D11` | `BuildingClass::Limbo @ 0x00445880` | DEC | building removed | Yes; sell/destroy/remove |
| `0x004D729A` | `FootClass::Unlimbo @ 0x004D7170` | INC | foot object enters map cell | Yes; normal unit spawn/unlimbo |
| `0x004DB2D7` | `FootClass::Limbo @ 0x004DB260` | DEC | foot object leaves map cell | Yes; normal remove/limbo |
| `0x004D86D8` | `FootClass::PerCellProcess @ 0x004D85D0` | DEC | moving foot vacates old source cell | Yes; normal movement |
| `0x004D8745` | `FootClass::PerCellProcess @ 0x004D85D0` | INC | moving foot claims new source cell | Yes; normal movement |
| `0x004CEDA4` | `FlyLocomotionClass::Descent_Step @ 0x004CE840` | DEC | landing/descent transition removes old adjacency influence | Yes conditional; aircraft landing/descent |
| `0x004CEE18` | `FlyLocomotionClass::Descent_Step @ 0x004CE840` | INC | landing/descent transition claims adjacency | Yes conditional; aircraft landing/descent |
| `0x004CEE8B` | `FlyLocomotionClass::Descent_Step @ 0x004CE840` | INC | second landing/descent claim path | Yes conditional; aircraft landing/descent |
| `0x0071C9A6` | `TerrainClass::Limbo @ 0x0071C930` | DEC | terrain object removed | Yes; trees/rocks are standard map objects |
| `0x0071D085` | `TerrainClass::Unlimbo @ 0x0071D000` | INC | terrain object placed | Yes; standard map load/runtime terrain objects |

Assembly evidence: `get_assembly_context` around every site above shows the same `CALL 0x005657A0`, `MOV DL, byte ptr [EAX+0x122]`, `INC/DEC DL`, loop counter compare against `0x8`, and store to `[EAX+0x122]`.

**Tiny detail W1 — the source cell itself is not incremented by its own writer.**  
Active in YR: **Yes.** Each writer loops over 8 neighbor cells of the source cell, not the source cell. A cell's count means "how many blocking/occupying objects are adjacent to me", not "am I occupied".

**Tiny detail W2 — the field is global CellClass state, not ground/bridge layer state.**  
Active in YR: **Yes.** The RMW sites write `byte [CellClass + 0x122]` after `MapClass::Get_Cell_By_Coord`; no writer context from the observed sites selects `FirstObject` vs `AltObject` or a movement layer. The reader also reads only the candidate `CellClass` byte before later CanEnter/cost logic. For flat Rust parity, counts must not be derived from only `MovementLayer::Ground` if bridge-layer occupants are otherwise present; either include all CellClass occupants or explicitly keep such cases out of the flat activation claim.

**Tiny detail W3 — the binary uses byte INC/DEC, not saturating arithmetic.**  
Active in YR: **Yes.** Assembly context shows `INC DL` / `DEC DL` and writes `DL` back. There is no clamp branch. Normal retail occupancy likely keeps values far below overflow, but a Rust implementation should not claim `u8::MAX` saturation is binary behavior.

## 4. Rust Surface Mapping

Current relevant Rust surfaces:

- `src/sim/pathfinding/core.rs`: `BlockerNeighborCounts` and `HierarchyGate` already implement the read-side boolean exception.
- `src/sim/movement/bump_crush.rs`: `build_entity_block_sets` produces structure hard blocks and a `LayeredEntityBlockMap` for unit soft blockers.
- `src/sim/movement/movement_path.rs`: flat path fallback currently passes `entity_blocks` and `entity_block_map`, but no blocker-neighbor counts.
- `src/map/resolved_terrain.rs`: `ResolvedTerrainCell` carries `overlay_blocks` and `terrain_object_blocks`, which are the closest Rust surfaces for wall-overlay and terrain-object writer sources.

Verified mapping:

1. Buildings: `BuildingClass::Unlimbo/Limbo` writer semantics map to Rust structure footprint hard-block cells from `build_entity_block_sets` / production building footprint helpers. Active in YR: **Yes**.
2. Units: `FootClass::Unlimbo/Limbo/PerCellProcess` writer semantics map to current unit occupancy/soft blocker cells, including moving friendly, stationary friendly, and enemy units. Active in YR: **Yes**.
3. Walls and blocking overlays: `OverlayClass::Mark` and overlay removal writers map to wall/blocking overlay cells, not ore. Current Rust should source these from resolved terrain overlay/object flags, not from `LayeredEntityBlockMap`. Active in YR: **Yes**.
4. Terrain objects: `TerrainClass::Unlimbo/Limbo` maps to `terrain_object_blocks`. Active in YR: **Yes**.
5. Aircraft descent writers exist and are active conditionally, but flat ground hierarchy activation can leave air/landing parity as a later extension if no flat caller currently models those object cells. Active in YR: **Conditional** on aircraft landing/descent.

## 5. Implementation Handoff

1. **Verified behavior:** `+0x122` is a global 8-neighbor object/blocker refcount read as zero/nonzero by hierarchical A*.  
   **Rust delta:** add a production `BlockerNeighborCounts` builder that increments all 8 in-bounds neighbors of each verified source blocker cell.  
   **Affected surface:** `src/sim/pathfinding/core.rs`, caller in `src/sim/movement/movement_path.rs`.  
   **Acceptance scenario:** a candidate off marker but adjacent to a blocker source is expanded; the same candidate with no adjacent blocker source is pruned.  
   **Proposed test:** `blocker_neighbor_counts_from_sources_allows_off_marker_adjacent_cell`.  
   **Risk:** medium; wrong source set changes flat route selection.

2. **Verified behavior:** wall overlays and terrain objects write the same counter as buildings/units; ore and ordinary base impassable terrain do not.  
   **Rust delta:** include `ResolvedTerrainCell.overlay_blocks || terrain_object_blocks` source cells, but do not include all `!PathGrid::is_walkable` cells.  
   **Affected surface:** `src/map/resolved_terrain.rs` access from movement/pathfinding count builder.  
   **Acceptance scenario:** wall/tree/rock blocker cells create neighbor counts; water/cliff-only unwalkable terrain does not create counts unless represented by a terrain object/overlay writer source.  
   **Proposed test:** `blocker_neighbor_counts_include_static_objects_not_plain_unwalkable_terrain`.  
   **Risk:** high; using all unwalkable cells would make broad water/cliff edges look like object-neighbor corridors.

3. **Verified behavior:** the field is not layer-scoped.  
   **Rust delta:** do not build counts from only the selected movement layer. For strict flat parity, include all current CellClass occupant/object blocker sources; if bridge-layer occupants are excluded, keep that as an explicit high-bridge/layered non-claim.  
   **Affected surface:** `LayeredEntityBlockMap` iteration or upstream count construction in `src/sim/movement/bump_crush.rs`.  
   **Acceptance scenario:** a flat hierarchy search sees nonzero neighbor count caused by an adjacent soft blocker entry regardless of whether that blocker would be a hard block in flat A*.  
   **Proposed test:** `blocker_neighbor_counts_are_global_not_selected_layer_only`.  
   **Risk:** medium; layer-filtered counts under-allow off-marker candidates near stacked/bridge occupants.

## 6. Negative Facts / Do Not Do

- Do not source counts from `UnitClass::Can_Enter_Cell`; the reader is `AStar_main_loop @ 0x00429A90`, not `UnitClass::Can_Enter_Cell`. Evidence: existing report disassembly audit plus live reader decompile and assembly at `0x00429EB1`.
- Do not treat `+0x122` as fog, shroud, water, amphibious, ore, bridge state, or zone type. Evidence: writer set is wall/building/foot/aircraft/terrain object RMW; reader is only hierarchical A*.
- Do not use every non-walkable `PathGrid` cell as a source blocker. Evidence: writers are object/overlay/terrain-object lifecycle paths; there is no base terrain/cliff/water writer in the xref set.
- Do not keep production counts layer-scoped and call that parity. Evidence: all writer/read sites touch the global `CellClass+0x122` byte.
- Do not document saturating arithmetic as binary behavior. Evidence: assembly uses raw `INC DL`/`DEC DL`; if Rust saturates for scale safety, document it as a deliberate scale exception or prove overflow unreachable.

## 7. Remaining Uncertainty

- Exact treatment of aircraft descent source cells in Rust is not mapped to a current flat movement caller. This is conditional and not needed to enable normal ground flat hierarchy if aircraft/landing paths stay outside the activation claim.
- Whether Rust currently preserves a convenient static-object source after dynamic wall destruction/terrain removal needs implementation-time source inspection. The binary behavior is verified; the Rust data plumbing may need a runtime overlay/object blocker source rather than only load-time `ResolvedTerrainGrid`.

## 8. Stale-Doc Replacement Wording

**Doc:** `C:/Users/enok/Documents/ra2-rust-game-docs/CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md`

Replace the Rust implication equivalent to "minor pathfinding speed differences only; no change in final path correctness" with:

> For hierarchical path parity, `CellClass+0x122` is route-selection relevant, not merely a performance optimization. It gates off-marker A* neighbor expansion during the hierarchical retry path. A Rust hierarchy activation must supply an equivalent object/blocker-neighbor count or leave hierarchy-gated A* disabled; missing or incorrectly sourced counts can change no-path vs retry behavior and selected detours.

## 9. Status

**COMPLETE** for the scoped writer/read/timing contract and Rust handoff. The binary side is resolved; Rust implementation should proceed only after adding the static object blocker source to the count builder design.
