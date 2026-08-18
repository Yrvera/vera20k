# Cell Occupancy Ordering Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make Rust `OccupancyGrid` preserve gamemd-compatible `CellClass::AddContent` ordering for each logical cell layer.

**Architecture:** This is a sim-only deterministic cache change. `OccupancyGrid` remains the owner of per-cell layer order, movement/spawn/unload/surface call sites pass the insertion class explicitly, and order-sensitive consumers use a layer-order iterator instead of reconstructing their own category order.

**Design Doc:** `docs/plans/2026-05-14-cell-occupancy-ordering-design.md`

---

## Grounding Summary

- `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md` is **High** confidence and says the relevant paths are active in YR.
- Live Ghidra re-check of `CellClass__AddContent` at `0x0047e8a0` confirms the selected list is `FirstObject` for ground and `AltObject` for bridge, with `WhatAmI == 6` appended to a non-empty selected list and other objects prepended.
- Live Ghidra re-check of `CellClass__RemoveContent` at `0x0047ea90` confirms unlinking preserves the relative order of remaining occupants.
- `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` is **High** confidence and confirms the current Rust category mapping: `UnitClass::WhatAmI == 1`, `AircraftClass::WhatAmI == 2`, `BuildingClass::WhatAmI == 6`, and `InfantryClass::WhatAmI == 0xF`.
- Live assembly at `TechnoClass__EnterCell_AddToMultiCells` call `0x005684bb` and `TechnoClass__ExitCell_RemoveFromMultiCells` call `0x005688eb` confirms `AddContent`/`RemoveContent` receive their layer selector from `object+0x8C`, not from a fresh per-cell bridge calculation.
- Live Ghidra re-check of `UnitClass__Can_Enter_Cell` at `0x0073f0a0` confirms passability scans the selected `+0xE4` or `+0xE8` list head-to-tail and has order-sensitive return/update branches.
- Live Ghidra re-check of `AStar_compute_edge_cost` at `0x00429830` confirms moving-friendly cost prediction starts from the selected list head.
- Live Ghidra follow-up corrected one misleading helper label: `CellClass__FindFirstBuilding` at `0x0047eba0` returns first `WhatAmI == 1`; the verified building helper is `Look_up_building_in_cell` at `0x0047c520`, which returns first `WhatAmI == 6` from the ground list.
- `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md` is **High** confidence and confirms ground/bridge lists are active in normal YR pathing.
- No cited report flags this occupancy behavior as TS-only legacy. The bridge/ground list split is active in standard YR.
- Current Rust `src/sim/occupancy.rs` stores a single `Vec<CellOccupant>` per cell and appends every add/move, which preserves deterministic membership but not gamemd list order.
- `EntityStore` is a `BTreeMap`, so `OccupancyGrid::rebuild` is deterministic but only replays stable-id order; it cannot recover runtime linked-list history after movement.
- Existing architecture pattern to mirror: `OccupancyGrid` stays in `sim/`, movement uses `MovementLayer`, and `world_spawn.rs` already resolves structure foundation cells through `building_footprint_cells`.
- Existing INI parsing already covers `Foundation`, `AddOccupyN`, and `RemoveOccupyN`; no new INI key parsing is required for this plan.
- Still unknown: exact save/load parity for runtime occupant history. This plan leaves `Simulation.occupancy` as a rebuilt cache and calls out save/load order as a separate snapshot-format decision.

## Key Technical Decisions

- Keep `CellOccupancy { occupants: Vec<CellOccupant> }`: Small per-cell occupancy counts make indexed insertion acceptable, and a `Vec` preserves deterministic scan order. **Confidence:** high
  - **Source:** design doc, `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`, repo pattern `src/sim/occupancy.rs`
- Make insertion explicit with `CellListInsertion`: The grid cannot safely infer building-vs-non-building from `sub_cell` or id shape, and signature changes force every add/move site to choose. **Confidence:** high
  - **Source:** design doc, live Ghidra `CellClass__AddContent` at `0x0047e8a0`
- Map `EntityCategory::Structure` to append and every other current category to prepend: verified `WhatAmI` values are `Unit=1`, `Aircraft=2`, `Building=6`, and `Infantry=0xF`; only `WhatAmI == 6` takes the append path. **Confidence:** high
  - **Source:** `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`, live Ghidra `0x00746e20`, raw assembly `0x0041c180`, live Ghidra `0x00459ec0`, live Ghidra `0x00523340`, `src/map/entities.rs`
- Treat Rust `MovementLayer` as the implementation-side equivalent of the gamemd selected-list flag for this narrow ordering change: `AddContent`/`RemoveContent` use `object+0x8C`, while `UnitClass::Can_Enter_Cell` can derive a consumer layer through bridge traversal and target-height logic. This plan should use existing entity layer state consistently, and leave exact `object+0x8C` update tracing to a separate bridge investigation. **Confidence:** high for the narrow plan, medium for full bridge edge parity
  - **Source:** `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`, `TechnoClass__EnterCell_AddToMultiCells` call `0x005684bb`, `TechnoClass__ExitCell_RemoveFromMultiCells` call `0x005688eb`, `CheckBridgeTraversal` at `0x004d9c60`
- Keep `debug_assert_matches` membership-only: A rebuild from sorted entities cannot prove runtime linked-list order after movement. **Confidence:** high
  - **Source:** design doc, `src/sim/world/mod.rs` rebuild cache path, `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`
- Add `iter_layer` and make first-match consumers use it where parity depends on full list order. This improves first-match parity but is not a full port of `UnitClass::Can_Enter_Cell`, whose mixed-occupant loop combines early returns with result-code accumulation. **Confidence:** high
  - **Source:** live Ghidra `UnitClass__Can_Enter_Cell` at `0x0073f0a0`, live Ghidra `AStar_compute_edge_cost` at `0x00429830`, current `src/sim/pathfinding/cell_entry.rs`

## Open Questions

### Resolved During Planning

- Does this need new INI parsing? No. The only INI-driven footprint data touched by this change is already parsed and merged: `Foundation`, `AddOccupyN`, and `RemoveOccupyN`.
- Is bridge-vs-ground list selection active in normal YR? Yes. The coordinate/elevation report and live `UnitClass__Can_Enter_Cell` check both confirm selected-list scanning.
- Which current Rust categories append? Only `EntityCategory::Structure`. The follow-up Ghidra report verifies `Unit=1`, `Aircraft=2`, `Building=6`, and `Infantry=0xF`; only `WhatAmI == 6` takes `AddContent`'s append branch.
- Does `AddContent`/`RemoveContent` recompute bridge layer from the cell? No. Their callers push `byte ptr [object+0x8C]`, so call sites should pass the entity's resolved occupancy layer consistently.
- Is `CellClass__FindFirstBuilding @ 0x0047EBA0` a building helper? No. The name is misleading in this binary; it returns first `WhatAmI == 1`. Use `Look_up_building_in_cell @ 0x0047C520` for verified building lookup evidence.
- Should ordered rebuild replace incremental runtime order? No. Rebuild is deterministic but cannot recover historical remove/add order.

### Deferred To Implementation

- Which non-test call sites are still hidden behind helper functions after the signature change? The compiler will identify any missed `add`/`move_entity` calls once Task 2 changes the API.
- Does changing `find_primary_blocker` from blockers-first to `iter_layer` expose existing tests that assumed the old approximation? Implementation should update only tests whose expected blocker choice was tied to the old Rust-only order, and should not claim full `UnitClass::Can_Enter_Cell` parity from this one helper.
- Should save/load serialize ordered occupancy directly or store per-occupant ordering metadata? This belongs with snapshot-format design because `Simulation.occupancy` is currently `#[serde(skip)]`.
- Exact write/update sites for gamemd `object+0x8C` bridge state are not required for this plan. They belong to a follow-up bridge-layer investigation before claiming full bridge edge-case parity.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/occupancy.rs` | Define insertion enum, layer-order iterator, insertion logic, rebuild semantics, and focused unit tests. |
| Modify | `src/sim/world/world_spawn.rs` | Pass structure append/non-structure prepend during map and production spawns. |
| Modify | `src/sim/movement/movement_step.rs` | Pass non-building prepend during normal cell transitions. |
| Modify | `src/sim/movement/movement_tick.rs` | Pass non-building prepend during drive-track cell jumps. |
| Modify | `src/sim/movement/tunnel_movement.rs` | Pass category-derived insertion when surfaced entities re-enter ground occupancy. |
| Modify | `src/sim/movement/teleport_movement.rs` | Pass category-derived insertion when teleport movement changes cells. |
| Modify | `src/sim/passenger.rs` | Pass non-building prepend when unloaded passengers are restored to the map. |
| Modify | `src/sim/production/production_sell.rs` | Pass non-building prepend when sell/destruction eject restores passengers to the map. |
| Modify | `src/sim/pathfinding/cell_entry.rs` | Add consumer-facing regression for full layer-order primary blocker selection. |
| Modify | Test files under `src/sim/**` | Update direct `OccupancyGrid::add` calls to pass explicit test insertion. |

## Interface Changes

- Create `pub enum CellListInsertion { PrependNonBuilding, AppendBuilding }` in `src/sim/occupancy.rs`.
- Add `CellListInsertion::from_category(category: EntityCategory) -> Self`.
- Change `OccupancyGrid::add` to require `insertion: CellListInsertion`.
- Change `OccupancyGrid::move_entity` to require `insertion: CellListInsertion`.
- Add `CellOccupancy::iter_layer(layer: MovementLayer) -> impl Iterator<Item = &CellOccupant> + '_`.
- Keep `blockers()` and `infantry()` public, but document that they are filtered views over current layer order.

## Sim Checklist

- [ ] All math uses existing integer/order operations; no f32/f64 in game logic.
- [ ] No new state is added to deterministic entity state.
- [ ] `OccupancyGrid` remains in `sim/` with no dependency on render/ui/sidebar/audio/net.
- [ ] Tick ordering impact is limited to within-cell occupant scan order after add/move/rebuild.
- [ ] `EntityStore` `BTreeMap` rebuild order is considered and documented as deterministic but not full runtime history parity.

## Risk Areas

- API churn is intentional and will touch many call sites. The compiler should be used as the first missed-site detector.
- Consumer order overrides can hide the new grid ordering. First-match logic must use `iter_layer` instead of separately scanning blockers then infantry.
- Structure footprint cells must all use append semantics, including cells produced by `AddOccupy` and `RemoveOccupy`.
- Bridge layer selection must keep using existing entity `MovementLayer` values from locomotor/path logic, while recognizing this is Rust's current approximation of gamemd's `object+0x8C` list selector.
- `find_primary_blocker` is a targeted approximation. Gamemd's full `UnitClass::Can_Enter_Cell` loop can keep scanning after some occupants and return early for others; this plan only fixes the first selected-object order used by the current Rust helper.
- Save/load remains a parity risk because the occupancy cache is skipped during serialization and rebuilt from entity state.
- Prepending shifts a small `Vec`; normal per-cell occupancy counts are tiny, so no linked-list rewrite is planned.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | Non-building add/move prepends within the selected layer | Newest moving units/infantry must be scanned first in crowded cells, affecting passability and pathing ties | Unit tests `non_buildings_prepend_on_same_layer` and `move_entity_reinserts_with_requested_order`; Ghidra `0x0047e8a0` |
| Task 2 | Building add appends within the selected layer | Building occupants should not become head blockers ahead of existing occupants after registration | Unit test `buildings_append_on_same_layer`; Ghidra `0x0047e8a0` |
| Task 3 | Ground and bridge order are independent | A bridge occupant must not reorder ground occupants in the same `(rx, ry)` cell | Unit test `layers_have_independent_order`; coordinate/elevation report |
| Task 8 | Primary blocker selection follows full layer order where the current Rust helper makes a first-match choice | `Can_Enter_Cell` and A* observe head-to-tail cell list order; the Rust helper should not impose a blockers-before-infantry priority when it picks one object | `cell_entry.rs` regression; Ghidra `0x0073f0a0` and `0x00429830`; note that full mixed-occupant parity remains out of scope |
| Task 9 | Runtime add/move sites all choose insertion class | A single append-only call site can silently reintroduce the parity gap | Compile failure after signature change plus targeted movement/spawn tests |

---

## Tasks

### Task 1: Define The Insertion Contract

**Why:** Establish the public API before changing insertion behavior or call sites.

**Files:**
- Modify: `src/sim/occupancy.rs`

**Pattern:** Mirrors existing small sim-side enums such as `MovementLayer`, kept close to the data structure that owns the contract.

**Step 1: Import `EntityCategory`**
```rust
use crate::map::entities::EntityCategory;
use crate::sim::movement::locomotor::MovementLayer;
```

**Step 2: Add the enum near `CellOccupant`**
```rust
/// Requested insertion order for a cell's selected gamemd object list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellListInsertion {
    PrependNonBuilding,
    AppendBuilding,
}
```

**Step 3: Add category mapping**
```rust
impl CellListInsertion {
    pub fn from_category(category: EntityCategory) -> Self {
        if category == EntityCategory::Structure {
            Self::AppendBuilding
        } else {
            Self::PrependNonBuilding
        }
    }
}
```

**Step 4: Verify**
Run: `cargo test -p vera20k occupancy -- --nocapture`
Expected: compile errors only at old `add`/`move_entity` call sites are acceptable after later signature edits; before signature edits, existing tests should still pass.

**Step 5: Commit**

### Task 2: Implement Layer-Local Insert Semantics

**Why:** This is the core gamemd-compatible `AddContent` behavior.

**Files:**
- Modify: `src/sim/occupancy.rs`

**Pattern:** Keep `Vec<CellOccupant>` and use deterministic positional insertion; no new storage abstraction.

**Step 1: Change `add` signature**
```rust
pub fn add(
    &mut self,
    rx: u16,
    ry: u16,
    entity_id: u64,
    layer: MovementLayer,
    sub_cell: Option<u8>,
    insertion: CellListInsertion,
)
```

**Step 2: Insert by selected layer**
```rust
let new_occupant = CellOccupant {
    entity_id,
    layer,
    sub_cell,
};
let occ = self.cells.entry((rx, ry)).or_default();
match insertion {
    CellListInsertion::PrependNonBuilding => {
        let index = occ
            .occupants
            .iter()
            .position(|o| o.layer == layer)
            .unwrap_or(0);
        occ.occupants.insert(index, new_occupant);
    }
    CellListInsertion::AppendBuilding => {
        let index = occ
            .occupants
            .iter()
            .rposition(|o| o.layer == layer)
            .map_or(occ.occupants.len(), |i| i + 1);
        occ.occupants.insert(index, new_occupant);
    }
}
```

**Step 3: Change `move_entity` signature and call**
```rust
pub fn move_entity(
    &mut self,
    old_rx: u16,
    old_ry: u16,
    new_rx: u16,
    new_ry: u16,
    entity_id: u64,
    layer: MovementLayer,
    sub_cell: Option<u8>,
    insertion: CellListInsertion,
) {
    self.remove(old_rx, old_ry, entity_id);
    self.add(new_rx, new_ry, entity_id, layer, sub_cell, insertion);
}
```

**Step 4: Update `rebuild`**
```rust
let insertion = CellListInsertion::from_category(entity.category);
grid.add(rx, ry, sid, layer, sub, insertion);
```

**Step 5: Verify**
Run: `cargo test -p vera20k occupancy -- --nocapture`
Expected: remaining failures are from call sites/tests not yet passing `CellListInsertion`.

**Step 6: Commit**

### Task 3: Add Layer-Order Iterator And Occupancy Unit Tests

**Why:** Consumers need a clear way to scan a selected gamemd list without reconstructing type order.

**Files:**
- Modify: `src/sim/occupancy.rs`

**Pattern:** Mirrors `blockers()` and `infantry()` returning filtered iterators over the internal `Vec`.

**Step 1: Add `iter_layer`**
```rust
/// All occupants on a selected movement layer in gamemd list order.
pub fn iter_layer(&self, layer: MovementLayer) -> impl Iterator<Item = &CellOccupant> + '_ {
    self.occupants.iter().filter(move |o| o.layer == layer)
}
```

**Step 2: Update filtered iterator comments**
```rust
/// Non-infantry occupants on a given layer, preserving layer-list order.
```
```rust
/// Infantry occupants on a given layer, preserving layer-list order.
```

**Step 3: Replace old occupancy tests with explicit insertion**
```rust
grid.add(
    5,
    5,
    1,
    MovementLayer::Ground,
    None,
    CellListInsertion::PrependNonBuilding,
);
```

**Step 4: Add `non_buildings_prepend_on_same_layer`**
```rust
let mut grid = OccupancyGrid::new();
grid.add(5, 5, 1, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 2, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
let ids: Vec<u64> = grid.get(5, 5).unwrap().iter_layer(MovementLayer::Ground).map(|o| o.entity_id).collect();
assert_eq!(ids, vec![2, 1]);
```

**Step 5: Add `buildings_append_on_same_layer`**
```rust
let mut grid = OccupancyGrid::new();
grid.add(5, 5, 1, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 100, MovementLayer::Ground, None, CellListInsertion::AppendBuilding);
grid.add(5, 5, 2, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
let ids: Vec<u64> = grid.get(5, 5).unwrap().iter_layer(MovementLayer::Ground).map(|o| o.entity_id).collect();
assert_eq!(ids, vec![2, 1, 100]);
```

**Step 6: Add `layers_have_independent_order`**
```rust
let mut grid = OccupancyGrid::new();
grid.add(5, 5, 1, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 10, MovementLayer::Bridge, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 2, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 20, MovementLayer::Bridge, None, CellListInsertion::PrependNonBuilding);
let ground: Vec<u64> = grid.get(5, 5).unwrap().iter_layer(MovementLayer::Ground).map(|o| o.entity_id).collect();
let bridge: Vec<u64> = grid.get(5, 5).unwrap().iter_layer(MovementLayer::Bridge).map(|o| o.entity_id).collect();
assert_eq!(ground, vec![2, 1]);
assert_eq!(bridge, vec![20, 10]);
```

**Step 7: Add `remove_preserves_remaining_order`**
```rust
let mut grid = OccupancyGrid::new();
grid.add(5, 5, 1, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 2, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(5, 5, 3, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.remove(5, 5, 2);
let ids: Vec<u64> = grid.get(5, 5).unwrap().iter_layer(MovementLayer::Ground).map(|o| o.entity_id).collect();
assert_eq!(ids, vec![3, 1]);
```

**Step 8: Add `move_entity_reinserts_with_requested_order`**
```rust
let mut grid = OccupancyGrid::new();
grid.add(1, 1, 1, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.add(2, 2, 2, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
grid.move_entity(1, 1, 2, 2, 1, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
let ids: Vec<u64> = grid.get(2, 2).unwrap().iter_layer(MovementLayer::Ground).map(|o| o.entity_id).collect();
assert_eq!(ids, vec![1, 2]);
```

**Step 9: Add `rebuild_uses_category_insertion`**
```rust
let mut entities = crate::sim::entity_store::EntityStore::new();
let mut first = crate::sim::game_entity::GameEntity::test_default(1, "E1", "Allies", 5, 5);
first.category = EntityCategory::Infantry;
first.sub_cell = Some(2);
let mut second = crate::sim::game_entity::GameEntity::test_default(2, "HTNK", "Allies", 5, 5);
second.category = EntityCategory::Unit;
let mut structure = crate::sim::game_entity::GameEntity::test_default(100, "GAPOWR", "Allies", 5, 5);
structure.category = EntityCategory::Structure;
entities.insert(first);
entities.insert(second);
entities.insert(structure);
let grid = OccupancyGrid::rebuild(&entities);
let ids: Vec<u64> = grid.get(5, 5).unwrap().iter_layer(MovementLayer::Ground).map(|o| o.entity_id).collect();
assert_eq!(ids, vec![2, 1, 100]);
```

**Step 10: Verify**
Run: `cargo test -p vera20k occupancy -- --nocapture`
Expected: all occupancy tests pass after call-site fixes in later tasks; before those fixes, only unrelated compile errors remain.

**Step 11: Commit**

### Task 4: Update Spawn And Structure Footprint Call Sites

**Why:** Map and production spawns register the initial occupants and all structure foundation cells.

**Files:**
- Modify: `src/sim/world/world_spawn.rs`

**Pattern:** Use the existing `category`/`spawn_category` values already captured before inserting entities.

**Layer note:** Use the already-resolved `spawn_layer` for every cell registered for the entity. This mirrors gamemd `AddContent` receiving the object's current `object+0x8C` list state, rather than recomputing bridge status separately for each footprint cell.

**Step 1: Import `CellListInsertion`**
```rust
use crate::sim::occupancy::CellListInsertion;
```

If `world_spawn.rs` currently imports only `foundation_dimensions`, expand the production helper import to include the adjusted footprint helper:
```rust
use crate::sim::production::{building_footprint_cells, foundation_dimensions};
```

**Step 2: Update map-spawn structure cells**
```rust
let insertion = CellListInsertion::from_category(category);
if let Some(cells) = spawn_cells {
    for (rx, ry) in cells {
        self.occupancy
            .add(rx, ry, spawn_sid, spawn_layer, None, insertion);
    }
} else {
    self.occupancy.add(
        spawn_rx,
        spawn_ry,
        spawn_sid,
        spawn_layer,
        spawn_sub_cell,
        insertion,
    );
}
```

**Step 3: Update production-spawn structure cells**
```rust
let insertion = CellListInsertion::from_category(spawn_category);
if spawn_category == EntityCategory::Structure {
    let cells = building_footprint_cells(
        spawn_rx,
        spawn_ry,
        &obj.foundation,
        &obj.add_occupy,
        &obj.remove_occupy,
    );
    for (rx, ry) in cells {
        self.occupancy
            .add(rx, ry, stable_id, spawn_layer, None, insertion);
    }
} else {
    self.occupancy.add(
        spawn_rx,
        spawn_ry,
        stable_id,
        spawn_layer,
        spawn_sub_cell,
        insertion,
    );
}
```

**Step 4: Verify**
Run: `cargo test -p vera20k world_spawn -- --nocapture`
Expected: spawn tests compile and pass; structure footprint occupancy still covers all existing footprint cells.

**Step 5: Commit**

### Task 5: Update Movement Transition Call Sites

**Why:** Moving non-building objects must be reinserted newest-first in the destination selected layer.

**Files:**
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Use category values already available as `category`, `snap.category`, or `entity.category`.

**Layer note:** Pass the transition's resolved `active_layer`. Do not recompute the destination layer inside `OccupancyGrid`; gamemd's add/remove layer selector is object state passed by the caller.

**Step 1: Import `CellListInsertion` in both files**
```rust
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
```

**Step 2: Update `movement_step.rs` normal crossing**
```rust
let insertion = CellListInsertion::from_category(category);
occupancy.move_entity(
    old_rx,
    old_ry,
    nx,
    ny,
    entity_id,
    active_layer,
    *sub_cell,
    insertion,
);
```

**Step 3: Update `movement_tick.rs` drive-track cell jump**
```rust
let insertion = CellListInsertion::from_category(entity.category);
occupancy.move_entity(
    old_rx,
    old_ry,
    nx,
    ny,
    entity_id,
    active_layer,
    entity.sub_cell,
    insertion,
);
```

**Step 4: Verify**
Run: `cargo test -p vera20k movement -- --nocapture`
Expected: movement tests compile and pass, with any changed blocker-order assertions adjusted only if they were asserting old append-only behavior.

**Step 5: Commit**

### Task 6: Update Special Movement And Restore-To-Map Sites

**Why:** Tunnel surfacing, teleport movement, unloading, and garrison ejection all call `AddContent` equivalents when an entity appears in a cell.

**Files:**
- Modify: `src/sim/movement/tunnel_movement.rs`
- Modify: `src/sim/movement/teleport_movement.rs`
- Modify: `src/sim/passenger.rs`
- Modify: `src/sim/production/production_sell.rs`

**Pattern:** Use the entity category available at the point the entity is restored. Pass `PrependNonBuilding` directly for passenger-only paths where the restored object is infantry.

**Layer note:** Use the restored entity's resolved layer at the add/move site. Passenger restore paths are ground-layer infantry in the current Rust model and therefore pass `MovementLayer::Ground` plus `PrependNonBuilding`.

**Step 1: Import `CellListInsertion` in each file with an add/move call**
```rust
use crate::sim::occupancy::CellListInsertion;
```

**Step 2: Update tunnel surfacing**
```rust
let insertion = CellListInsertion::from_category(entity.category);
occupancy.add(
    entity.position.rx,
    entity.position.ry,
    id,
    MovementLayer::Ground,
    entity.sub_cell,
    insertion,
);
```

**Step 3: Update teleport `move_entity` calls**
```rust
let insertion = CellListInsertion::from_category(entity.category);
occupancy.move_entity(
    old_rx,
    old_ry,
    new_rx,
    new_ry,
    id,
    new_layer,
    entity.sub_cell,
    insertion,
);
```

**Step 4: Update passenger unload**
```rust
sim.occupancy.add(
    exit_rx,
    exit_ry,
    pax_id,
    MovementLayer::Ground,
    pax_sub_cell,
    CellListInsertion::PrependNonBuilding,
);
```

**Step 5: Update production sell/destruction ejection**
```rust
sim.occupancy.add(
    spawn_rx,
    spawn_ry,
    pax_id,
    MovementLayer::Ground,
    pax_sub_cell,
    CellListInsertion::PrependNonBuilding,
);
```

**Step 6: Verify**
Run:
```powershell
cargo test -p vera20k passenger -- --nocapture
cargo test -p vera20k production_sell -- --nocapture
cargo test -p vera20k teleport_movement -- --nocapture
cargo test -p vera20k tunnel_movement -- --nocapture
```
Expected: tests compile and pass; restored infantry occupy cells as before but now scan newest-first.

**Step 7: Commit**

### Task 7: Update Remaining Direct Test And Helper Calls

**Why:** The explicit API should leave no old append-only test helpers or fixture setup behind.

**Files:**
- Modify: any files reported by `rg -n "add\\(|move_entity\\(" src/sim` after Tasks 4-6

**Pattern:** Test fixtures should pass explicit insertion values matching what they are modeling.

**Step 1: Find remaining old calls**
```powershell
rg -n "occupancy\.add|\.add\([^;\n]*MovementLayer|move_entity\(" src/sim
```

**Step 2: Update non-structure fixture occupants**
```rust
occupancy.add(
    rx,
    ry,
    entity_id,
    MovementLayer::Ground,
    sub_cell,
    CellListInsertion::PrependNonBuilding,
);
```

**Step 3: Update structure fixture occupants**
```rust
occupancy.add(
    rx,
    ry,
    structure_id,
    MovementLayer::Ground,
    None,
    CellListInsertion::AppendBuilding,
);
```

**Step 4: Update test helper tuple signatures only where useful**
```rust
fn make_occ(entries: &[(u16, u16, u64, MovementLayer, Option<u8>, CellListInsertion)]) -> OccupancyGrid {
    let mut grid = OccupancyGrid::new();
    for &(rx, ry, id, layer, sub_cell, insertion) in entries {
        grid.add(rx, ry, id, layer, sub_cell, insertion);
    }
    grid
}
```

**Step 5: Verify**
Run: `cargo test -p vera20k --no-run`
Expected: no compile errors for `OccupancyGrid::add` or `OccupancyGrid::move_entity` arity.

**Step 6: Commit**

### Task 8: Make Primary Blocker Approximation Use Layer Order

**Why:** `find_primary_blocker` is the first consumer regression from the design: it currently scans blockers before infantry, which can override gamemd list order for the current Rust helper's first-match behavior. This does not make the helper a full `UnitClass::Can_Enter_Cell` port.

**Files:**
- Modify: `src/sim/pathfinding/cell_entry.rs`

**Pattern:** Use the new `CellOccupancy::iter_layer` and keep existing `mover_bypass_grid` structure skipping.

**Step 1: Rewrite primary scan**
```rust
for occupant in occ.iter_layer(layer) {
    if occupant.entity_id == mover_id {
        continue;
    }
    if mover_bypass_grid
        && entities
            .get(occupant.entity_id)
            .is_some_and(|e| e.category == EntityCategory::Structure)
    {
        continue;
    }
    return Some(occupant.entity_id);
}
None
```

**Step 2: Preserve missing-entity behavior**
```rust
if mover_bypass_grid
    && entities
        .get(occupant.entity_id)
        .is_some_and(|e| e.category == EntityCategory::Structure)
{
    continue;
}
```
This keeps missing entities as blockers, matching the previous `unwrap_or(true)` behavior when `mover_bypass_grid` was active.

**Step 3: Update existing test setup imports**
```rust
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
```

**Step 4: Add `find_primary_blocker_follows_layer_order`**
```rust
let mut occ = OccupancyGrid::new();
occ.add(5, 5, 10, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
occ.add(5, 5, 20, MovementLayer::Ground, Some(2), CellListInsertion::PrependNonBuilding);

let mut entities = EntityStore::new();
let mut blocker = GameEntity::test_default(10, "HTNK", "Allies", 5, 5);
blocker.category = EntityCategory::Unit;
entities.insert(blocker);
let mut infantry = GameEntity::test_default(20, "E1", "Allies", 5, 5);
infantry.category = EntityCategory::Infantry;
entities.insert(infantry);

let result = find_primary_blocker(
    (5, 5),
    MovementLayer::Ground,
    42,
    false,
    &occ,
    &entities,
);
assert_eq!(result, Some(20));
```

**Step 5: Verify**
Run: `cargo test -p vera20k find_primary_blocker -- --nocapture`
Expected: existing bypass-grid structure test still passes, and the new regression proves newest-prepended infantry can be the first blocker.

**Step 6: Commit**

### Task 9: Audit Other Order-Sensitive Consumers

**Why:** The new grid order only helps parity when consumers do not manually override it.

**Files:**
- Inspect: `src/sim/movement/bump_crush.rs`
- Inspect: `src/sim/movement/scatter.rs`
- Inspect: `src/sim/pathfinding/terrain_speed.rs`
- Inspect: `src/sim/smudge_grid.rs`

**Pattern:** Only change first-match or capped-list consumers. Count/boolean consumers can keep `blockers()`, `infantry()`, `has_blockers_on()`, and `count_on()`.

**Step 1: Search consumers**
```powershell
rg -n "blockers\(|infantry\(|has_blockers_on|count_on\(|is_empty_on\(" src/sim
```

**Step 2: Classify each hit**
```text
count/boolean: no change
first-match: use iter_layer
capped collection: use iter_layer
category-specific semantic query: keep blockers/infantry
```

**Step 3: Document unchanged risk in a short code comment only where a consumer intentionally remains category-specific**
```rust
// Category-specific crush checks intentionally scan infantry and blockers separately;
// full CellClass list-order parity for area/capped scans is tracked by the occupancy plan.
```

**Step 4: Verify**
Run:
```powershell
cargo test -p vera20k movement -- --nocapture
cargo test -p vera20k pathfinding -- --nocapture
```
Expected: no behavioral regressions outside consumers intentionally changed in Task 8.

**Step 5: Commit**

### Task 10: Full Verification And Plan Closure

**Why:** This change touches a shared sim cache and many movement/spawn call sites.

**Files:**
- No source edits unless verification exposes a missed call site or test expectation tied to old append-only order.

**Pattern:** Compile all tests first, then run focused suites that exercise occupancy, movement, spawn, unload, and pathfinding.

**Step 1: Compile all tests**
```powershell
cargo test -p vera20k --no-run
```
Expected: no compile errors.

**Step 2: Run focused tests**
```powershell
cargo test -p vera20k occupancy -- --nocapture
cargo test -p vera20k cell_entry -- --nocapture
cargo test -p vera20k movement_occupancy -- --nocapture
cargo test -p vera20k movement_step -- --nocapture
cargo test -p vera20k world_spawn -- --nocapture
cargo test -p vera20k passenger -- --nocapture
cargo test -p vera20k production_sell -- --nocapture
```
Expected: all focused tests pass.

**Step 3: Run broader sim tests**
```powershell
cargo test -p vera20k sim -- --nocapture
```
Expected: all sim tests pass, or unrelated pre-existing failures are recorded with file/test names.

**Step 4: Run formatting**
```powershell
cargo fmt
```
Expected: no unintended churn outside edited Rust files.

**Step 5: Run final search**
```powershell
rg -n "occupancy\.add|\.add\([^;\n]*MovementLayer|move_entity\(" src/sim
```
Expected: every `OccupancyGrid::add` and `move_entity` call passes `CellListInsertion`.

**Step 6: Commit**

## Sources & References

- **Design doc:** `docs/plans/2026-05-14-cell-occupancy-ordering-design.md`
- **Ghidra reports:** `docs/research/CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`
- **Ghidra reports:** `docs/research/CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`
- **Ghidra reports:** `docs/research/COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`
- **Live gamemd.exe verification:** `CellClass__AddContent` at `0x0047e8a0`
- **Live gamemd.exe verification:** `CellClass__RemoveContent` at `0x0047ea90`
- **Live gamemd.exe verification:** `TechnoClass__EnterCell_AddToMultiCells` call at `0x005684bb` pushes `byte ptr [object+0x8C]` into `AddContent`
- **Live gamemd.exe verification:** `TechnoClass__ExitCell_RemoveFromMultiCells` call at `0x005688eb` pushes `byte ptr [object+0x8C]` into `RemoveContent`
- **Live gamemd.exe verification:** `UnitClass__WhatAmI` at `0x00746e20`, `AircraftClass::WhatAmI` raw assembly at `0x0041c180`, `BuildingClass__WhatAmI` at `0x00459ec0`, `InfantryClass__WhatAmI` at `0x00523340`
- **Live gamemd.exe verification:** `UnitClass__Can_Enter_Cell` at `0x0073f0a0`
- **Live gamemd.exe verification:** `AStar_compute_edge_cost` at `0x00429830`
- **Live gamemd.exe verification:** `CellClass__FindFirstBuilding` at `0x0047eba0` is a misleading label and returns first `WhatAmI == 1`; `Look_up_building_in_cell` at `0x0047c520` returns first `WhatAmI == 6`
- **Live gamemd.exe verification:** `CheckBridgeTraversal` at `0x004d9c60`
- **INI keys:** `art.ini` / `artmd.ini` `Foundation=`, `AddOccupy1..8=`, `RemoveOccupy1..8=`; already parsed in `src/rules/art_data.rs`
- **Related code:** `src/sim/occupancy.rs`
- **Related code:** `src/sim/pathfinding/cell_entry.rs`
- **Related code:** `src/sim/movement/movement_step.rs`
- **Related code:** `src/sim/movement/movement_tick.rs`
- **Related code:** `src/sim/world/world_spawn.rs`
- **Related code:** `src/sim/production/production_tech.rs`
- **Recent commits checked:** `2b9f0cf pathfinding+occupancy: stamp building footprints via building_footprint_cells (respects AddOccupy/RemoveOccupy)`, `7f3191b sim/movement_step: update bridge-resolver call site to new signature`, `69369cf sim/movement: thread BridgeStateUpdate through CrossingOutput + drive-track render`
