# Cell Occupancy Ordering - Ghidra Research Report

**Address(es):** `0x0047E8A0`, `0x0047EA90`, `0x0073F0A0`, `0x00429830`, `0x00481670`, `0x00489280`, `0x0047C3D0`, `0x0047EBA0`, `0x0047C520`  
**Confidence:** High  
**Active in YR:** Yes

## 1. Overview

gamemd.exe does not treat per-cell occupants as an unordered set. Each `CellClass` has two singly-linked occupant lists:

- `CellClass+0xE4`: ground `FirstObject`
- `CellClass+0xE8`: bridge/alternate `AltObject`

Insertion order is type-sensitive: buildings are appended to the selected list tail, while most other objects are prepended to the selected list head. Multiple gameplay systems then traverse these lists head-to-tail. Some consumers accumulate all occupants, but others stop at the first matching object, limit collection to 10 entries, or keep the first object on distance ties. Therefore, cell occupant ordering is a real parity surface.

## 2. Core Binary Findings

### 2.1 AddContent selects ground vs bridge list, then inserts with type-specific order

**Function:** `CellClass__AddContent` at `0x0047E8A0`  
**Active in YR:** Yes.

Layer selection:

```c
if (bridge_arg == 0) {
    head = this->FirstObject; // +0xE4
} else {
    head = this->AltObject;   // +0xE8
}
```

If object pointer is null, the function returns immediately.

Then:

```c
what = object->WhatAmI(); // vtable+0x2C

if (what == 6 && head != NULL) {
    // append object at tail
    while (head->NextObject != NULL) {
        head = head->NextObject;
    }
    head->NextObject = object;
    object->NextObject = NULL;
} else {
    // prepend object at head of selected layer list
    object->NextObject = head;
    selected_layer_head = object;
}
```

Offsets:

| Offset | Meaning |
|--------|---------|
| `CellClass+0xE4` | ground list head |
| `CellClass+0xE8` | bridge/alt list head |
| `ObjectClass+0x30` | next pointer in cell list |

**Key detail:** `WhatAmI == 6` objects are appended only when the selected list is non-empty. If the selected list is empty, the same object becomes the head through the normal path. In practice, this means later non-building occupants can still be prepended in front of existing buildings.

### 2.2 RemoveContent unlinks from the selected list and preserves remaining order

**Function:** `CellClass__RemoveContent` at `0x0047EA90`  
**Active in YR:** Yes.

The function uses the same bridge boolean to choose `+0xE4` or `+0xE8`, then:

- if the target is the selected list head, advances the head to `object->NextObject`,
- otherwise walks until `prev->NextObject == object`,
- patches `prev->NextObject = object->NextObject`,
- clears `object->NextObject = NULL`.

This preserves the relative order of all remaining occupants.

### 2.3 Moving non-building objects become newest-first in their destination cell

The AddContent rule means a moving vehicle/infantry/object that enters a cell is normally inserted at the **front** of the selected ground/bridge list. A building/structure is normally inserted at the **tail** if another occupant is already present.

Observed consequence:

```text
gamemd non-building move into occupied cell: newest entrant is scanned first
gamemd building registration into occupied cell: building is scanned after existing occupants
```

This differs from an append-only vector unless the Rust code deliberately reproduces the order when adding/moving occupants.

## 3. Consumers Where Ordering Matters

### 3.1 UnitClass::Can_Enter_Cell scans one selected list head-to-tail

**Function:** `UnitClass__Can_Enter_Cell` at `0x0073F0A0`  
**Active in YR:** Yes - standard UnitClass A* passability check.

After bridge/height logic chooses the layer:

```c
if (bridge_layer_flag == 0) {
    obj = cell->FirstObject; // +0xE4
} else {
    obj = cell->AltObject;   // +0xE8
}

while (obj != NULL) {
    ... classify object ...
    obj = obj->NextObject; // +0x30
}
```

Most branches accumulate the most severe return code seen so far, but there are also early returns:

- self/mission-target cases can return `0`,
- passenger-sharing cases can return `0`,
- some blocked weapon/crush cases return `7`,
- some building/garrison cases return `0`, `3`, `5`, or `7`,
- head-on moving-friendly logic can return `7`.

**Finding:** Ordering can matter when a cell contains multiple mixed occupants, because the first object that triggers an early return wins.

### 3.2 AStar_compute_edge_cost uses the selected list head as the first moving-friendly blocker

**Function:** `AStar_compute_edge_cost` at `0x00429830`  
**Active in YR:** Yes - core A* cost function.

For Can_Enter_Cell return code `2` (moving friendly), the function starts from the selected list head:

```c
obj = bridge_flag ? cell->AltObject : cell->FirstObject;
```

It then predicts the blocker path up to 10 cells. The first candidate object in the list is the one whose path is followed. If the first object is not active, cost becomes `4.0`. If its path leads to empty terrain or no path direction, cost can remain `1.0`. If the chain remains blocked for 10 steps, cost becomes `4.0`.

**Finding:** When several moving friendlies share one cell, occupant order can change A* cost: the binary predicts the head object's future path, not a sorted or nearest blocker set.

### 3.3 Scatter_Objects walks list order and collects up to 10 occupants

**Function:** `CellClass__Scatter_Objects` at `0x00481670`  
**Active in YR:** Yes - called from locomotor/unit processing.

The function:

1. selects `+0xE4` or `+0xE8`,
2. optionally does an eligibility scan,
3. re-selects the same list,
4. walks head-to-tail and stores occupants into a temporary array,
5. calls each occupant's `Scatter` method in array order.

Important details:

- Collection is list-order based.
- The temporary array is capped by the function's internal selection logic; prior docs identify the scatter call surface as up to 10 occupants.
- With `force != 0`, the initial eligibility gate is skipped and occupants are scattered unconditionally.

**Finding:** Scatter dispatch order follows cell-list order. If Rust iterates a different order, units can receive scatter orders in a different sequence.

### 3.4 Apply_area_damage collects targets in list order before applying damage

**Function:** `Apply_area_damage` at `0x00489280`  
**Active in YR:** Yes - warhead area damage path.

Per affected cell, the function chooses the list:

```c
obj = search_above_bridge ? cell->AltObject : cell->FirstObject;
for (; obj != NULL; obj = obj->NextObject) {
    ... filter target ...
    damage_vector.push({ object, distance });
}
```

Later it iterates the collected vector and calls the damage vtable on each target.

**Finding:** Damage target discovery order is list order. Distance is stored beside the object, but the observed decompilation does not sort targets before applying damage. Side-effect order may therefore differ when multiple objects are in one cell or same area pass.

### 3.5 CellClass::Find_Nearest_Object keeps first object on distance ties

**Function:** `CellClass__Find_Nearest_Object` at `0x0047C3D0`  
**Active in YR:** Yes.

The function scans selected list head-to-tail and updates the best object only when:

```c
candidate_distance < best_distance
```

It does not replace on equal distance.

**Finding:** Ties are resolved by list order. The first equally-near object wins.

### 3.6 First-matching helper functions return list-order matches

Relevant helpers:

- `CellClass__FindFirstBuilding` label at `0x0047EBA0`
- `Look_up_building_in_cell` at `0x0047C520`

Both walk a cell object list through `ObjectClass+0x30` and return the first object whose `WhatAmI` matches the helper's target class. `Look_up_building_in_cell` scans the ground list (`+0xE4`) and returns the first `WhatAmI == 6` object.

**Finding:** These helpers are first-match functions. Their return value is order-sensitive.

## 4. Current Rust Implementation Status

### 4.1 OccupancyGrid appends all occupants

**File:** `src/sim/occupancy.rs`

Current `OccupancyGrid::add`:

```rust
let occ = self.cells.entry((rx, ry)).or_default();
occ.occupants.push(CellOccupant { ... });
```

Current `move_entity` performs:

```rust
remove(old_cell, entity_id);
add(new_cell, entity_id, layer, sub_cell);
```

So Rust's default behavior is:

```text
all added occupants append to the end of the Vec
```

### 4.2 Rebuild order is stable-id order

`OccupancyGrid::rebuild` scans `EntityStore::values()`. `EntityStore` is `BTreeMap<u64, GameEntity>`, so rebuild inserts in ascending stable-id order.

This is deterministic, but it is not automatically gamemd's linked-list order. gamemd's order is a function of AddContent calls over time plus the prepend-vs-append rule.

### 4.3 Rust consumers already observe vector order

Examples:

| Rust surface | Behavior |
|--------------|----------|
| `CellOccupancy::blockers(layer)` | filters `occupants.iter()` in Vec order |
| `CellOccupancy::infantry(layer)` | filters `occupants.iter()` in Vec order |
| `pathfinding::cell_entry::find_primary_blocker` | uses first blocker, then first non-self infantry |
| `movement::bump_crush::collect_crush_victims` | collects infantry first, then blockers |

**Finding:** Rust already exposes occupancy order to gameplay decisions, but that order is not currently the same rule as gamemd's cell list.

## 5. Parity Assessment

### What matches

- Rust now has a layer-aware occupancy model, which conceptually matches `FirstObject` vs `AltObject`.
- Queries can filter by `MovementLayer::Ground` or `MovementLayer::Bridge`.
- The data structure is deterministic.

### What does not match yet

| Topic | gamemd.exe | Current Rust |
|-------|------------|--------------|
| Non-building insertion into occupied cell | prepend to list head | append to Vec tail |
| Building insertion into occupied list | append to tail | append to Vec tail |
| Move into destination cell | remove old, then usually prepend new object | remove old, then append |
| Rebuild/deserialization order | implied by original AddContent history | stable-id sorted insertion |
| Query order | linked-list head-to-tail | Vec insertion order |
| Multiple-occupant tie cases | first in linked list wins | first in Vec wins |

### Practical severity

This is usually invisible for simple cases:

- one vehicle per cell,
- one building foundation occupant,
- infantry subcells where only counts matter,
- empty/blocked boolean checks.

It becomes visible in crowded or edge cases:

- multiple infantry in one cell,
- friendly moving blockers in one cell,
- crush/scatter decisions,
- cell damage affecting multiple occupants,
- nearest-object ties,
- bridge cells with separate ground/bridge occupants,
- any function that stops on first matching occupant.

## 6. Recommended Implementation Direction

This report does not implement anything, but the parity target is clear:

1. Keep the clean Rust `OccupancyGrid` structure if desired.
2. Make insertion semantics match gamemd:
   - non-building/non-structure occupants should be inserted at the front of their layer order,
   - buildings/structures should be inserted at the back of their layer order.
3. Preserve that order when moving/removing.
4. Avoid relying on `rebuild` stable-id order as a parity replacement unless rebuild also reconstructs gamemd-equivalent layer order.
5. Where a Rust consumer intentionally needs a different semantic order, document that difference as a conscious abstraction rather than accidental Vec order.

## 7. Open Questions

1. **Exact object class mapping for every Rust `EntityCategory`:** We need a table mapping Rust categories to gamemd `WhatAmI` values for insertion ordering. The important verified case is `WhatAmI == 6` using tail insertion in `AddContent`.
2. **Infantry subcell ordering:** gamemd also has ground/bridge occupation bitmasks (`CellClass+0x124/+0x128`). This report covers object-list order, not subcell allocation priority.
3. **Rebuild after save/load:** gamemd may serialize/restore cell lists directly or reconstruct via object registration. Rust rebuild currently sorts by stable id. Save/load parity needs a targeted check.
4. **Target acquisition docs conflict:** Some older docs appear to swap `+0xE4/+0xE8` names. The Ghidra-verified meaning used here is `+0xE4 = ground FirstObject`, `+0xE8 = bridge AltObject`.

## 8. Answer to the Current Question

No: our current Rust occupancy ordering should not be assumed identical to gamemd.exe.

The high-level layer model matches, but **ordering within a cell is a parity gap**:

```text
gamemd: non-buildings prepend, buildings append
Rust: all occupants append
```

Because multiple active binary systems traverse from the list head, this can affect observable behavior in crowded cells and bridge edge cases.

## Sources

- Ghidra decompiled:
  - `CellClass__AddContent` at `0x0047E8A0`
  - `CellClass__RemoveContent` at `0x0047EA90`
  - `UnitClass__Can_Enter_Cell` at `0x0073F0A0`
  - `AStar_compute_edge_cost` at `0x00429830`
  - `CellClass__Scatter_Objects` at `0x00481670`
  - `Apply_area_damage` at `0x00489280`
  - `CellClass__Find_Nearest_Object` at `0x0047C3D0`
  - `CellClass__FindFirstBuilding` label at `0x0047EBA0`
  - `Look_up_building_in_cell` at `0x0047C520`
- Existing docs referenced:
  - `CELLCLASS_STRUCT_GHIDRA_REPORT.md`
  - `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`
  - `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
  - `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md`
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md`
  - `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`
- Rust files checked:
  - `src/sim/occupancy.rs`
  - `src/sim/entity_store.rs`
  - `src/sim/pathfinding/cell_entry.rs`
  - `src/sim/movement/bump_crush.rs`
  - `src/sim/movement/movement_occupancy.rs`
  - `src/sim/world/world_spawn.rs`
