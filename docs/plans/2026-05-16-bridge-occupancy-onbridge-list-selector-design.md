# Bridge Occupancy OnBridge List Selector Design

## Goal

Make Rust high-bridge occupancy/list membership use `GameEntity::on_bridge` as the authoritative ground-vs-bridge object-list selector, matching gamemd.exe `ObjectClass+0x8C`, while keeping path/locomotor layer separate.

## Architecture Context

`OccupancyGrid` is the sim-side deterministic cache for per-cell occupants. It stores a single `Vec<CellOccupant>` per `(rx, ry)` with `MovementLayer` tags, and the already-implemented occupancy-ordering work makes layer-local order match gamemd `CellClass::AddContent`: non-buildings prepend and structures append.

The missing parity contract is the layer source. gamemd does not use path layer or locomotor layer to pick `CellClass+0xE4` versus `CellClass+0xE8` for normal add/remove. `TechnoClass__EnterCell_AddToMultiCells` and `TechnoClass__ExitCell_RemoveFromMultiCells` push `object+0x8C` into `AddContent` / `RemoveContent`. In Rust, `GameEntity::on_bridge` is the field documented as that persistent bridge flag, while `LocomotorState.layer` follows A* path layer and is intentionally allowed to disagree on ramps.

Current relevant data flow:

```text
PathGrid/A* path_layers
  -> MovementTarget::layer_at(next_index)
  -> LocomotorState.layer / active_layer for movement and pathing
  -> movement_bridge.rs computes BridgeStateUpdate
  -> apply_pending_bridge_render_state updates on_bridge and bridge_occupancy
```

Current mismatch points:

- `OccupancyGrid::rebuild` derives occupancy layer from `locomotor.layer`.
- `movement_step.rs` calls `occupancy.move_entity(... active_layer ...)` before bridge state is resolved.
- `movement_tick.rs` resolves bridge state before its drive-track occupancy move, but still passes `active_layer`.
- `bridge_orchestrator.rs::drop_in_bridge_deck_entities` clears `on_bridge` and locomotor state but does not relayer the occupancy cache.
- `bump_crush.rs::build_entity_block_sets` keys the `LayeredEntityBlockMap` from `movement_layer_or_ground`, which can return `Bridge` from `loco.layer` even when `on_bridge=false`.

Relevant existing patterns:

- `movement_bridge.rs` already decouples `on_bridge` from `loco.layer`; this design extends that distinction into occupancy membership.
- `CellListInsertion::from_category` already expresses add-order policy; this design adds a separate list-layer policy.
- `LayeredEntityBlockMap` already represents separate ground/bridge soft blockers and should receive the same selected object-list layer as persistent occupancy.

## Impact Analysis

Files expected to change:

- `src/sim/game_entity.rs`: add an explicit helper for occupiable ground/bridge list layer, or equivalent central helper in `occupancy.rs`.
- `src/sim/occupancy.rs`: use the helper in `rebuild`; add tests for disagreement cases.
- `src/sim/movement/movement_step.rs`: choose add layer from projected post-transition `on_bridge`, not `active_layer`, and preserve remove-before-update/add-after-update order.
- `src/sim/movement/movement_tick.rs`: use projected post-transition `on_bridge` for drive-track occupancy moves.
- `src/sim/world/bridge_orchestrator.rs`: relayer bridge-deck occupants to ground occupancy during `DropIn`.
- `src/sim/movement/bump_crush.rs`: use the same list-layer helper when building `LayeredEntityBlockMap`.
- Tests in `src/sim/occupancy.rs`, `src/sim/movement/*`, `src/sim/world/bridge_orchestrator.rs`, and possibly `src/sim/world/world_tests.rs`.

What depends on this:

- Movement legality and deferred cell checks.
- A* moving-friendly soft-block cost prediction.
- Bridge ramp movement where `loco.layer` and `on_bridge` intentionally disagree.
- Bridge collapse `DropIn` behavior.
- Debug/rebuild paths that reconstruct occupancy from entities.

Sim and determinism impact:

- No new authoritative state is required; this uses existing `on_bridge`, `bridge_occupancy`, and locomotor state.
- No floating point math is introduced.
- No render/UI/audio/net dependencies are introduced.
- Tick ordering changes only for occupancy cache insertion relative to bridge-state projection; entity state application remains compatible with the existing `BridgeStateUpdate` flow.
- `EntityStore` deterministic iteration remains unchanged.

Risk areas:

- A helper named too broadly could be reused for pathing decisions and re-collapse `on_bridge` with `loco.layer`. The helper must be named as an occupancy/list selector.
- Removing and adding occupancy in a different order can expose stale assumptions in reservation and sub-cell code.
- `DropIn` relayering must handle same-cell move from bridge to ground without losing sub-cell or order semantics.
- Non-drive locomotor families are not in this scope; accidental broad changes to aircraft/jumpjet/carryall behavior should be avoided.

## Chosen Approach

Use a single explicit object-list layer helper and thread it through rebuild, movement insertion, bridge collapse relayering, and A* soft-block map construction.

The helper should return the layer used for `FirstObject`/`AltObject` style membership, not the path layer:

```rust
pub fn occupancy_list_layer(&self) -> Option<MovementLayer> {
    let motion_layer = self
        .locomotor
        .as_ref()
        .map_or(MovementLayer::Ground, |l| l.layer);
    if matches!(motion_layer, MovementLayer::Air | MovementLayer::Underground) {
        return None;
    }
    Some(if self.on_bridge {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    })
}
```

The exact location can be `GameEntity` or `occupancy.rs`; the contract matters more than the file. The recommended location is `GameEntity` because callers already hold entities, and the method can sit beside `movement_layer_or_ground` with comments that distinguish path layer from object-list layer.

Movement code should project the post-transition `on_bridge` before adding to occupancy:

```text
old_on_bridge = entity/snapshot on_bridge
remove from old cell
move coordinates
bridge_update = resolve_cell_transition_bridge_state(...)
new_on_bridge = apply BridgeStateUpdate to old_on_bridge without mutating entity yet
new_list_layer = Bridge if new_on_bridge else Ground
add to new cell using new_list_layer
later apply_pending_bridge_render_state mutates entity.on_bridge/bridge_occupancy
```

For drive-track code that already resolves `BridgeStateUpdate` before `move_entity`, replace `active_layer` in the occupancy move with the projected object-list layer.

For `DropIn`, same-cell relayering should use existing grid operations:

```text
for each deck entity in cell:
  clear entity.on_bridge / bridge_occupancy / set loco.layer=Ground
  occupancy.move_entity(rx, ry, rx, ry, id, Ground, sub_cell, insertion)
```

That mirrors gamemd's observable result: the object stops blocking the bridge-deck list and becomes a ground-list occupant after `DropIn`.

## Tiny-Detail Ledger

- `+0xE4` is the ground object-list head and `+0xE8` is the bridge/deck object-list head. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md]
- Normal `AddContent` / `RemoveContent` list selection comes from the caller's list-layer argument, normally `ObjectClass+0x8C` / `OnBridge`, not from path layer. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md; Ghidra 0x005683C0, 0x005687F0]
- `UnitClass::Can_Enter_Cell` can choose object-list layer before `CheckBridgeTraversal` and occupancy-bit layer after it; those two layers can disagree. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md; Ghidra 0x0073F0A0]
- Ground/under-bridge objects and bridge-deck objects do not block each other through the normal selected-list scan. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md]
- Normal transition order is remove with old `OnBridge`, move coordinates, evaluate bridge predicate, update `OnBridge`, add with new `OnBridge`. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md; BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md]
- `LocomotorState.layer` and `on_bridge` intentionally disagree on ramps: going up can have `loco.layer=Bridge` while `on_bridge=false`; going down can have `loco.layer=Ground` while `on_bridge=true`. [doc: 2026-05-11-bridge-locomotor-layer-correctness-design.md]
- Non-buildings prepend and buildings append within the selected list; changing layer source must preserve existing `CellListInsertion` behavior. [doc: CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md; 2026-05-14-cell-occupancy-ordering-design.md]
- `DropIn` sets falling bytes, unmarks, removes from display, clears `OnBridge`, submits, and marks again. Rust must at least relayer persistent occupancy after clearing `on_bridge`. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md; Ghidra 0x005F4160]
- Bridge collapse damages ground-list occupants with C4Warhead and calls `DropIn` for bridge-list occupants. This design must not merge the two lists before collapse handling. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md; Ghidra 0x0047DD70]
- Air and underground entities are currently excluded from Rust occupancy; this design preserves that for the scoped drive/walk/ship high-bridge paths. [repo: src/sim/occupancy.rs]
- Low bridge / `TubeClass` behavior is separate and must not be routed through high-bridge `AltObject` list semantics. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md]
- Non-drive `ObjectClass+0x8C` writer families remain partially scoped and are deferred, not silently generalized. [doc: BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md Remaining Open Questions]

## Design

### Components

**Occupancy list layer helper**

Add an explicit helper, recommended on `GameEntity`:

```rust
pub fn occupancy_list_layer(&self) -> Option<MovementLayer>
```

Contract:

- Returns `Some(Bridge)` when the entity is an occupiable object and `on_bridge=true`.
- Returns `Some(Ground)` when the entity is an occupiable ground/bridge object and `on_bridge=false`, even if `loco.layer == Bridge`.
- Returns `None` for Air and Underground locomotor layers in the current scoped implementation.
- Does not replace `movement_layer_or_ground`; that existing helper remains path/current-movement oriented and should not be used for object-list membership.

**Projected bridge-state helper**

Add a small pure helper near `BridgeStateUpdate`:

```rust
pub(super) fn projected_on_bridge(current: bool, update: BridgeStateUpdate) -> bool {
    match update {
        BridgeStateUpdate::Set(_) => true,
        BridgeStateUpdate::Clear => false,
        BridgeStateUpdate::Unchanged => current,
    }
}
```

This avoids mutating the entity early just to choose the occupancy insertion layer.

**OccupancyGrid usage**

Keep `OccupancyGrid` representation unchanged. The layer tag stored on `CellOccupant` becomes the object-list layer, not the path layer. `CellListInsertion` remains unchanged.

**LayeredEntityBlockMap usage**

`build_entity_block_sets` should use `entity.occupancy_list_layer()` instead of `entity.movement_layer_or_ground()` so A* soft blockers are keyed by the same selected object-list layer used by occupancy.

### Interfaces / Contracts

- `MovementLayer` continues to carry both path-layer and object-list-layer values in different contexts. The code must name variables clearly:
  - `active_layer`, `next_layer`, and `loco.layer` are path/movement layers.
  - `occupancy_layer`, `old_list_layer`, `new_list_layer`, and `object_list_layer` are `FirstObject`/`AltObject` selectors.
- `OccupancyGrid::add` and `move_entity` continue to accept a `MovementLayer`; callers are responsible for passing the object-list layer.
- `GameEntity::movement_layer_or_ground` should not be changed in this design except possibly to clarify comments. It is used by command/pathing code and changing it broadly risks unintended path changes.
- `GameEntity::occupancy_list_layer` is the only intended source for rebuild and blocker-map list membership.
- Movement-step code that needs the post-transition layer should use `projected_on_bridge(old_on_bridge, bridge_update)`.

### Data Flow

Normal movement crossing:

```text
snapshot old cell and old_on_bridge
resolve can-enter context using path layer, as today
apply cell coordinate transition
compute bridge_update from src/dst cells
project new_on_bridge from old_on_bridge + bridge_update
new_list_layer = Bridge if projected true else Ground
occupancy.move_entity(old_cell, new_cell, entity_id, new_list_layer, sub_cell, insertion)
reserve destination/update sub-cell
later apply_pending_bridge_render_state mutates entity.on_bridge and bridge_occupancy
```

Drive-track crossing:

```text
move coordinates
compute bridge_update
project new_on_bridge from current entity.on_bridge + bridge_update
occupancy.move_entity(..., projected list layer, ...)
apply_pending_bridge_render_state
```

Rebuild:

```text
for entity in EntityStore order:
  if inside transport: skip
  if entity.occupancy_list_layer() is None: skip
  add entity to layer with category-derived insertion
```

Bridge collapse `DropIn`:

```text
collect ids where entity is on bridge deck in target cell
for each id:
  clear bridge_occupancy/on_bridge, set z to ground, set loco.layer Ground
  move occupancy entry from bridge layer to ground layer in same cell
```

Soft blocker map:

```text
for entity in entities:
  layer = entity.occupancy_list_layer()
  if None: skip
  insert hard/soft blocker into ground/bridge map by that layer
```

### Error Handling

- If movement projection produces `None` for a mover that is already tracked in occupancy, treat it as a development invariant violation in debug tests. The scoped drive/walk/ship paths should always project Ground or Bridge.
- Rebuild should skip `None` exactly as it currently skips Air/Underground.
- `DropIn` should not fail if occupancy lacks the entity; `OccupancyGrid::remove` is already no-op if not found. Tests should assert the expected relayering in normal cases.
- If `path_grid` is missing and `BridgeStateUpdate::Unchanged` is returned, projected `on_bridge` preserves the current state. That matches the existing fallback behavior and avoids inventing a bridge transition.

### Testing Strategy

Unit tests in `src/sim/game_entity.rs` or `src/sim/occupancy.rs`:

- `occupancy_list_layer_uses_on_bridge_over_loco_bridge`: `on_bridge=false`, `loco.layer=Bridge` returns Ground.
- `occupancy_list_layer_uses_on_bridge_over_loco_ground`: `on_bridge=true`, `loco.layer=Ground` returns Bridge.
- `occupancy_list_layer_skips_air_and_underground`: current Air/Underground exclusions remain.

Unit tests in `src/sim/occupancy.rs`:

- `rebuild_uses_on_bridge_for_layer`: entity with `on_bridge=true`, `loco.layer=Ground` rebuilds into Bridge.
- `rebuild_does_not_use_path_bridge_without_on_bridge`: entity with `on_bridge=false`, `loco.layer=Bridge` rebuilds into Ground.

Movement tests:

- Ramp-up crossing where `next_layer=Bridge` and `BridgeStateUpdate::Unchanged` must add occupancy to Ground.
- Ramp-down crossing where `next_layer=Ground` and `BridgeStateUpdate::Unchanged` must keep occupancy on Bridge.
- Enter body crossing where `BridgeStateUpdate::Set(deck)` adds occupancy to Bridge.
- Exit crossing where `BridgeStateUpdate::Clear` adds occupancy to Ground.

Drive-track tests:

- Same disagreement cases as movement-step, but through the drive-track branch in `movement_tick.rs`.

Bridge collapse tests:

- `drop_in_reprofiles_occupancy_to_ground`: bridge occupant in a cell is on Bridge before collapse and Ground after `drop_in_bridge_deck_entities`.
- Existing "ground occupant survives/dies" tests should verify ground occupants remain ground-list occupants and are not relayered as deck occupants.

Blocker-map tests:

- Entity with `on_bridge=false`, `loco.layer=Bridge` appears in ground `LayeredEntityBlockMap`.
- Entity with `on_bridge=true`, `loco.layer=Ground` appears in bridge `LayeredEntityBlockMap`.

Verification commands for the eventual implementation:

```powershell
cargo test -p vera20k occupancy -- --nocapture
cargo test -p vera20k movement_bridge -- --nocapture
cargo test -p vera20k movement_step -- --nocapture
cargo test -p vera20k bridge_orchestrator -- --nocapture
cargo test -p vera20k bump_crush -- --nocapture
cargo test -p vera20k world_tests bridge -- --nocapture
```

## Architectural Decisions

- Use `on_bridge` as the object-list selector because it is the existing Rust analog of gamemd `ObjectClass+0x8C`. This follows the verified binary and avoids creating duplicate bridge state.
- Keep `loco.layer` as the path/movement layer. It drives path walkability and can legitimately disagree with `on_bridge` on ramps.
- Do not change `MovementLayer` or split it into new enum types in this design. The code already uses `MovementLayer` pervasively; clear helper and variable names are enough for this targeted parity fix.
- Keep `OccupancyGrid` storage unchanged. The parity bug is call-site layer source and timing, not the vector representation.
- Preserve Air/Underground occupancy exclusion in this scoped implementation. Non-drive bridge writer families are deferred until researched or separately designed.
- Do not add new serialized state. The deterministic hash already includes `on_bridge` and `bridge_occupancy`; occupancy remains a cache.

Tech debt addressed:

- Removes the ambiguity of using `movement_layer_or_ground` for object-list membership.
- Aligns rebuild, runtime occupancy, bridge collapse relayering, and A* soft-block maps around one list-layer contract.

Tech debt intentionally left:

- Save/load exact within-cell list order remains limited by occupancy being a rebuilt cache.
- Explicit bitfield modeling for `+0x124` / `+0x128` remains out of scope.
- Non-drive `OnBridge` writer families remain deferred.

## Alternatives Considered

### Alternative A: Keep Using `movement_layer_or_ground`

This is the current drift. It is simple but wrong when `loco.layer` and `on_bridge` disagree. It produces bridge/ground blockers in the wrong selected list on ramp ticks and during rebuild. Rejected.

### Alternative B: Change `movement_layer_or_ground` To Always Return `on_bridge ? Bridge : Ground`

This would fix some occupancy call sites by accident, but it would also change command/pathing code that intentionally needs locomotor/path layer. That creates hidden coupling and risks breaking A* and movement behavior. Rejected.

### Alternative C: Split `MovementLayer` Into `PathLayer` And `ObjectListLayer`

This is the cleanest type-level model, but it would be a broad refactor through pathfinding, movement, and occupancy. The immediate parity fix is smaller: add an explicit helper and variable naming contract. Keep this as a possible future cleanup if layer confusion continues. Rejected for this fix.

### Alternative D: Chosen - Explicit Occupancy List Helper

This preserves current architecture, fixes the verified parity gaps, and makes the distinction visible at the call sites that matter. It is the best fit for a targeted implementation plan.

## Deferred Scope

- Teleport, hover, jumpjet, aircraft, carryall, and other non-drive `ObjectClass+0x8C` writer families. The verified report marks these as only partially scoped.
- Low bridge / `TubeClass` occupancy behavior. Verified separate from high-bridge `AltObject`.
- Exact runtime initializer formulas for `DAT_00B1D0AC` and `DAT_00AC13BC`.
- Future explicit `+0x124` / `+0x128` bitfield reservation model.
