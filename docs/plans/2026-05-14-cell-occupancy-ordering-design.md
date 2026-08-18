# Cell Occupancy Ordering - Design

## Goal

Make Rust `OccupancyGrid` preserve gamemd-compatible `CellClass::AddContent`
ordering for each logical cell layer:

- `CellClass+0xE4` ground `FirstObject` and `CellClass+0xE8` bridge
  `AltObject` are separate logical lists.
- Non-buildings prepend to the selected layer list.
- Buildings append to the selected layer list.
- Removal preserves the relative order of remaining occupants.

This is a parity surface because passability, scatter, A* moving-blocker cost,
area damage, nearest-object ties, and first-building helpers walk cell occupants
in linked-list order.

## Sources

- `docs/research/CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`
  - `CellClass__AddContent` at `0x0047E8A0`
  - `CellClass__RemoveContent` at `0x0047EA90`
  - order-sensitive consumers including `UnitClass__Can_Enter_Cell`,
    `AStar_compute_edge_cost`, `Scatter_Objects`, `Apply_area_damage`,
    `Find_Nearest_Object`
- `docs/research/COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`
  - verifies `+0xE4 = FirstObject` ground list and `+0xE8 = AltObject`
    bridge/alternate list
  - verifies bridge/ground list selection is active in normal YR pathing
- Current Rust:
  - [src/sim/occupancy.rs](../../src/sim/occupancy.rs)
  - [src/sim/pathfinding/cell_entry.rs](../../src/sim/pathfinding/cell_entry.rs)
  - [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs)
  - [src/sim/world/world_spawn.rs](../../src/sim/world/world_spawn.rs)

## Architecture Context

`OccupancyGrid` is a sim-side deterministic cache stored on `Simulation`. It is
maintained incrementally at spawn, movement, unloading/ejection, teleport, tunnel
surfacing, and death/despawn sites. It must stay in `sim/` and must not depend on
render, UI, audio, sidebar, or net.

The current grid stores one `Vec<CellOccupant>` per `(rx, ry)` with a
`MovementLayer` tag. That representation can remain, but its contract should be:
the meaningful order is the filtered order within a selected layer, not the
unfiltered all-layer `Vec` order.

Today `OccupancyGrid::add` appends every occupant, and `move_entity` removes then
appends. That matches buildings but is wrong for moving units and infantry, which
should become newest-first in their destination layer.

## Chosen Approach

Keep the single per-cell `Vec<CellOccupant>`, but define insertion in terms of
the selected `MovementLayer`:

- **Prepend:** insert before the first occupant with the same layer. If no
  occupant on that layer exists, insert at the front of the cell vector.
- **Append:** insert after the last occupant with the same layer. If no occupant
  on that layer exists, push to the end of the cell vector.

This preserves gamemd list order for `occ.iter_layer(Ground)` and
`occ.iter_layer(Bridge)` while avoiding a broader storage rewrite.

## API Decision

Make insertion class explicit. Do not let `OccupancyGrid` guess from `sub_cell`
or from id shape.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellListInsertion {
    PrependNonBuilding,
    AppendBuilding,
}

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

Update `OccupancyGrid::add` and `move_entity` to require the insertion value:

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
)
```

Call sites with an entity available should pass
`CellListInsertion::from_category(entity.category)`. Tests may pass the explicit
enum directly.

Add a layer-order iterator so order-sensitive consumers can avoid manually
combining `blockers()` and `infantry()` in a way that overrides gamemd order:

```rust
pub fn iter_layer(&self, layer: MovementLayer) -> impl Iterator<Item = &CellOccupant> + '_
```

Keep `blockers()` and `infantry()` for type-specific queries, but document that
they are filtered views over layer order, not a substitute for full list-order
scans when first-match parity matters.

## Rebuild Behavior

`OccupancyGrid::rebuild(&EntityStore)` should use the same category-derived
insertion rule, so map-load-like reconstruction is deterministic and follows the
same add semantics for the order it replays.

Do not treat rebuild as a complete parity substitute for runtime AddContent
history. Once units have moved, the exact gamemd list order depends on the
sequence of prior remove/add calls, and current `GameEntity` state does not store
that history. Since `Simulation.occupancy` is currently `#[serde(skip)]`, loading
a saved active game and rebuilding from sorted `EntityStore` can lose exact
within-cell order.

Decision:

- For normal runtime, the incrementally maintained grid is authoritative.
- For debug validation, `debug_assert_matches` may stay membership-only unless a
  future ordered-rebuild source exists.
- For exact save/load parity, add a follow-up design to either serialize
  ordered occupancy or persist enough per-entity cell-list order metadata to
  reconstruct it.

## Testing Strategy

Add focused unit tests in [src/sim/occupancy.rs](../../src/sim/occupancy.rs):

1. `non_buildings_prepend_on_same_layer`
   - Add ids 1 then 2 on ground with `PrependNonBuilding`.
   - `iter_layer(Ground)` returns `[2, 1]`.

2. `buildings_append_on_same_layer`
   - Add non-building 1, building 100, non-building 2 on ground.
   - `iter_layer(Ground)` returns `[2, 1, 100]`.

3. `layers_have_independent_order`
   - Mix ground and bridge occupants in one cell.
   - Ground order and bridge order each match their own prepend/append history.

4. `remove_preserves_remaining_order`
   - Remove a middle occupant from a mixed layer list.
   - Remaining layer order is unchanged.

5. `move_entity_reinserts_with_requested_order`
   - Move a non-building into an occupied destination.
   - Destination layer scans the moved entity first.

6. `rebuild_uses_category_insertion`
   - Build an `EntityStore` with two non-structures and one structure sharing a
     cell.
   - Rebuild and assert the order matches replaying those entities through
     category-derived insertion.

Add at least one consumer-facing regression test after the API lands:

- In `cell_entry.rs`, create two same-layer blockers and assert the first
  blocker chosen follows `iter_layer` order. This prevents a future refactor from
  silently restoring append-only behavior.

## Risk Areas

- **Consumer order overrides:** `find_primary_blocker` prefers blockers before
  infantry, and `collect_crush_victims` collects infantry before blockers. These
  may remain valid approximations for their specific logic, but they are not
  full gamemd list scans. Any first-match or capped collection behavior should
  use `iter_layer`.
- **Save/load:** rebuild after deserialization cannot recover runtime list
  history from current entity fields.
- **Call-site omissions:** changing `add`/`move_entity` signatures is intentional
  so every insertion site must choose building append vs non-building prepend.
- **Building footprint cells:** structures should pass `AppendBuilding` for every
  occupied foundation cell, including `AddOccupy`/`RemoveOccupy` adjusted cells.
- **Bridge layer state:** layer selection must continue to use
  `movement_layer_or_ground` or the existing resolved bridge layer, not raw
  height guesses.
- **Performance:** prepending in a small `Vec` shifts entries. Normal occupancy
  counts are tiny, so this is acceptable; do not introduce a linked-list or ECS
  structure for this.

## Deferred Questions

- Exact Rust mapping for unusual object categories beyond
  `EntityCategory::Structure => WhatAmI == 6`. Current categories imply every
  non-structure uses prepend.
- Whether save/load parity should serialize `OccupancyGrid` directly or store an
  order token per live occupant. This should be decided with the snapshot format,
  not hidden inside `rebuild`.
- Whether area damage, scatter, and nearest-object code should be audited
  immediately after the grid change or in separate parity passes.
