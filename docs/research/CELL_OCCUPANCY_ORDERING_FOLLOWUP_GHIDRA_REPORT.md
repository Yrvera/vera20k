# Cell Occupancy Ordering Follow-up - Ghidra Verification Report

**Date:** 2026-05-14  
**Scope:** Follow-up verification for the cell occupancy ordering implementation plan.  
**Primary questions:** `WhatAmI` mapping, `CellClass::AddContent`/`RemoveContent` layer argument flow, and whether `UnitClass::Can_Enter_Cell` mixed-occupant behavior depends on full list order.  
**Confidence:** High for the verified points below.  
**Active in YR:** Yes for the cell list maintenance and `UnitClass::Can_Enter_Cell` paths. Bridge-specific effects are conditional on bridge cell flags and target height.

## Executive Findings

The core ordering plan is directionally correct: gamemd maintains one linked list per cell layer, buildings append to the selected layer list, and current Rust append-only occupancy order does not preserve this behavior.

Three plan assumptions need tightening before implementation:

1. `WhatAmI` mapping for the current Rust categories is verified as:
   - `UnitClass::WhatAmI @ 0x00746E20` returns `1`.
   - `AircraftClass::WhatAmI @ 0x0041C180` returns `2` from raw assembly (`MOV EAX,0x2; RET`).
   - `BuildingClass::WhatAmI @ 0x00459EC0` returns `6`.
   - `InfantryClass::WhatAmI @ 0x00523340` returns `0xF`.

2. `CellClass::AddContent` and `CellClass::RemoveContent` do not derive the layer from the cell at the add/remove site. The bridge/alternate-list boolean is passed from the object's field at `object+0x8C` by `TechnoClass` enter/exit cell code.

3. `UnitClass::Can_Enter_Cell` scans the selected occupant list in head-to-tail order, but it is not equivalent to a simple "first occupant always wins" rule. Some occupants return immediately; many others update the current result code and continue. A Rust `find_primary_blocker` change to use list order is still useful, but it remains an approximation of the full gamemd routine.

## Verified Binary Evidence

### 1. `WhatAmI` Values

Verified functions:

| Class | Address | Result |
|-------|---------|--------|
| `UnitClass` | `0x00746E20` | `1` |
| `AircraftClass` | `0x0041C180` | `2` |
| `BuildingClass` | `0x00459EC0` | `6` |
| `InfantryClass` | `0x00523340` | `0xF` |

`AircraftClass::WhatAmI` was not present as a named Ghidra function in this session, but direct assembly at `0x0041C180` is:

```asm
0041c180: MOV EAX,0x2
0041c185: RET
```

Implication for Rust:

```text
EntityCategory::Unit      => WhatAmI 1   => prepend
EntityCategory::Aircraft  => WhatAmI 2   => prepend, if registered in a cell list
EntityCategory::Structure => WhatAmI 6   => append
EntityCategory::Infantry  => WhatAmI 0xF => prepend
```

Only `WhatAmI == 6` gets the append path in `CellClass::AddContent`. For the current Rust `EntityCategory` set, mapping `Structure` to append and all other categories to prepend is correct.

### 2. `CellClass::AddContent`

**Function:** `CellClass__AddContent @ 0x0047E8A0`

The function selects the linked-list head from the second stack argument:

```c
if (bridge_arg == 0) {
    head = this->FirstObject; // CellClass+0xE4
} else {
    head = this->AltObject;   // CellClass+0xE8
}
```

Then it calls `object->WhatAmI()` through vtable slot `+0x2C`. If the result is `6` and the selected list is non-empty, it walks `ObjectClass+0x30` to the tail and appends. Otherwise it prepends to the selected list head.

Important nuance: a building in an empty selected list becomes the head through the normal path. Later non-building occupants can still prepend in front of it.

### 3. `CellClass::RemoveContent`

**Function:** `CellClass__RemoveContent @ 0x0047EA90`

The function uses the same second stack argument to choose `FirstObject` (`+0xE4`) or `AltObject` (`+0xE8`). It unlinks the target from that list and clears the target's `ObjectClass+0x30` pointer. The relative order of all remaining occupants is preserved.

### 4. Add/Remove Layer Argument Source

`CellClass::AddContent` is called from `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`.

Assembly at the call:

```asm
005684b1: MOV DL,byte ptr [EDI + 0x8c]
005684b7: MOV ECX,EAX
005684b9: PUSH EDX
005684ba: PUSH EDI
005684bb: CALL 0x0047e8a0
```

`CellClass::RemoveContent` is called from `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`.

Assembly at the call:

```asm
005688e1: MOV DL,byte ptr [EDI + 0x8c]
005688e7: MOV ECX,EAX
005688e9: PUSH EDX
005688ea: PUSH EDI
005688eb: CALL 0x0047ea90
```

`EDI` is the object being added or removed. Therefore both operations use the object's own `+0x8C` bridge/alt-list state, not a fresh per-cell bridge calculation inside `AddContent` or `RemoveContent`.

Implementation implication:

- Rust insertion/removal must use the entity's current resolved occupancy list state.
- For buildings/foundations, the same object layer state applies across all registered footprint cells.
- Removal must use the same layer state that was used for insertion, or Rust can leave stale entries in the wrong layer.

### 5. Bridge Traversal Can Select a Different Consumer Layer

**Function:** `CheckBridgeTraversal @ 0x004D9C60`

This function can mutate both the target height and the local bridge-list flag used by `UnitClass::Can_Enter_Cell`.

Verified behavior:

- If target height is unknown and the target cell has bridge flag `0x100`, it can set target height to `cell.Level + 4`.
- If moving up a bridge height difference of `4`, and the source cell has both bridge flag `0x100` and bridgehead flag `0x200`, it writes `*param_4 = 1` and returns clear.
- It returns `7` for invalid bridge/ramp transitions.

`UnitClass::Can_Enter_Cell` also has a post-check overwrite of the occupancy bit snapshot when the resolved target height equals `cell.Level + 4`.

Implication:

The consumer's selected list can come from bridge/traversal logic, not from `AddContent` itself. The occupancy-order implementation plan does not need to solve all bridge height semantics, but any consumer call that passes a `MovementLayer` should avoid assuming that "requested movement layer" and gamemd's selected list flag are always identical in bridge edge cases.

## Mixed Occupant Behavior in `UnitClass::Can_Enter_Cell`

**Function:** `UnitClass__Can_Enter_Cell @ 0x0073F0A0`

The function selects one occupant list:

```c
obj = bridge_list_flag ? cell->AltObject : cell->FirstObject;
```

It then iterates via `ObjectClass+0x30`.

Ordering matters because the loop contains both early returns and result-code accumulation.

Verified early-return examples:

- Same transport destination can return `0`.
- Train-vs-train pass-through can return `0`.
- Mission enter/capture cases against the current navigation target building can return `0`.
- Own cargo on the unload target can return `7`.
- Stationary allied building returns `7`.
- Enemy occupant with no usable weapon/jumpjet path can return `7`.
- Enemy invisible building can return `7`.
- Moving friendly deadlock check can return `7`.

Verified accumulation examples:

- Enemy occupant can raise the result to `5`.
- Allied stationary non-building can raise the result to `6`.
- Moving friendly can raise the result to `2`.
- Garrison/scatter-related building cases can raise the result to `3` or `5`.
- Neutral/civilian mission state can raise the result to `1`.

Conclusion:

Full list order matters, but a single primary-blocker scan cannot fully reproduce gamemd. For the current implementation plan, replacing category-priority scans with `iter_layer` is still the right local improvement. It makes first-match decisions follow gamemd list order, but it should be documented as an approximation until `UnitClass::Can_Enter_Cell` is ported more directly.

## Helper Correction: Misleading `FindFirstBuilding` Label

The Ghidra label `CellClass__FindFirstBuilding @ 0x0047EBA0` is misleading for this binary. The live function scans the selected cell list and returns the first object whose `WhatAmI()` is `1`, not `6`.

Verified decompile:

```c
for (; obj != NULL; obj = obj->NextObject) {
    if (obj->WhatAmI() == 1) {
        return obj;
    }
}
```

The building lookup helper is `Look_up_building_in_cell @ 0x0047C520`; it scans the ground list and returns the first object whose `WhatAmI()` is `6`.

Implementation implication:

Existing research or plans should stop using `CellClass__FindFirstBuilding @ 0x0047EBA0` as proof of a building-first helper. It is still order-sensitive, but it is looking for units by verified `WhatAmI` value.

## Current Rust Impact

Observed Rust code:

- `src/map/entities.rs` defines `EntityCategory::{Unit, Infantry, Structure, Aircraft}`.
- `src/sim/occupancy.rs` stores `CellOccupancy { occupants: Vec<CellOccupant> }`.
- `OccupancyGrid::add` appends every occupant.
- `OccupancyGrid::move_entity` removes then calls append-style `add`.
- `OccupancyGrid::rebuild` scans deterministic `EntityStore` order but cannot recover runtime linked-list history.
- `CellOccupancy::blockers(layer)` and `CellOccupancy::infantry(layer)` reconstruct category priority rather than exposing one selected-list iterator.
- `src/sim/pathfinding/cell_entry.rs::find_primary_blocker` checks blockers before infantry.

Confirmed plan changes:

1. Add an explicit insertion class:

```rust
pub enum CellListInsertion {
    Prepend,
    Append,
}
```

2. Map categories as:

```rust
Structure => Append
Unit | Infantry | Aircraft => Prepend
```

3. Add `CellOccupancy::iter_layer(layer)` to expose gamemd list order per logical layer.

4. Update first-match consumers to use `iter_layer` rather than `blockers()` then `infantry()` where order parity matters.

5. Keep the Rust `Vec<CellOccupant>` storage shape. The binary requirement is observable traversal order, not a linked-list implementation.

Plan wording to revise:

- Replace any "category priority" framing with "selected list order, with only structure append vs non-structure prepend at insertion time."
- Add the `object+0x8C` source for add/remove layer selection.
- Warn that `MovementLayer` is a Rust approximation of gamemd's selected list flag in some bridge traversal cases.
- Mark `find_primary_blocker` as a local approximation, not a full port of `UnitClass::Can_Enter_Cell`.

## Recommended Next Investigation Before Implementation

No more Ghidra work is required to implement the narrow `OccupancyGrid` ordering change.

The next useful reverse-engineering task is separate and should not block this plan: trace how `object+0x8C` is set and updated during bridge entry/exit and height changes. That would tighten bridge-layer correctness for movement consumers, but the ordering fix can proceed with Rust's existing `MovementLayer` plumbing as long as tests make the current approximation explicit.

## Evidence Status

Verified from live Ghidra/decompilation or direct assembly in this session:

- `CellClass__AddContent @ 0x0047E8A0`
- `CellClass__RemoveContent @ 0x0047EA90`
- `TechnoClass__EnterCell_AddToMultiCells` call at `0x005684BB`
- `TechnoClass__ExitCell_RemoveFromMultiCells` call at `0x005688EB`
- `UnitClass__WhatAmI @ 0x00746E20`
- `AircraftClass::WhatAmI` raw assembly at `0x0041C180`
- `BuildingClass__WhatAmI @ 0x00459EC0`
- `InfantryClass__WhatAmI @ 0x00523340`
- `CheckBridgeTraversal @ 0x004D9C60`
- `UnitClass__Can_Enter_Cell @ 0x0073F0A0`
- `CellClass__FindFirstBuilding @ 0x0047EBA0`
- `Look_up_building_in_cell @ 0x0047C520`

Inference from verified behavior:

- `object+0x8C` is the object's current bridge/alternate cell-list state for `AddContent` and `RemoveContent`.
- Current Rust category-to-insertion mapping is safe for the existing four `EntityCategory` variants.
- Full `Can_Enter_Cell` parity needs more than a primary-blocker helper, but list-order iteration is the correct next step for the present implementation plan.
