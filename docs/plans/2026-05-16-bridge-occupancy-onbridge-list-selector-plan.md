# Bridge Occupancy OnBridge List Selector Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained. Do not broaden scope into non-drive locomotor families or low-bridge `TubeClass` behavior.

**Goal:** Make Rust high-bridge occupancy/list membership use `GameEntity::on_bridge` as the authoritative ground-vs-bridge object-list selector, matching gamemd.exe `ObjectClass+0x8C`, while keeping path/locomotor layer separate.

**Architecture:** This is a deterministic `sim/` parity fix. `OccupancyGrid`, movement crossing, bridge collapse, and A* soft-block maps stay in their current modules; the plan only changes which layer is passed to existing occupancy/list APIs. No render, UI, audio, net, serialization, or INI parser changes are required.

**Design Doc:** `docs/plans/2026-05-16-bridge-occupancy-onbridge-list-selector-design.md`

---

## Grounding Summary

- `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` is the primary source. It is GREEN after verify-doc and marks high confidence for `CellClass+0xE4/+0xE8`, `+0x124/+0x128`, list selection, `Can_Enter_Cell`, and collapse handling.
- Live Ghidra re-check this planning pass confirmed `CellClass::AddContent @ 0047E8A0` selects `FirstObject` versus `AltObject` from the stack list-layer byte and preserves selected-list insertion order.
- Live Ghidra re-check confirmed `ObjectClass::DropIn @ 005F4160` clears `ObjectClass+0x8C` after unmark/remove-from-display and before submit/mark.
- Live Ghidra re-check confirmed `CellClass::BlowUpBridge @ 0047DD70` damages `FirstObject` occupants and calls vtable `+0xEC` (`DropIn`) on `AltObject` occupants.
- Live Ghidra re-check confirmed `UnitClass::Can_Enter_Cell @ 0073F0A0` selects one object list (`+0xE4` or `+0xE8`) and can separately re-snapshot bridge occupancy bits after `CheckBridgeTraversal`.
- Current Rust has the needed state: `GameEntity::on_bridge`, `BridgeOccupancy`, `LocomotorState.layer`, and `OccupancyGrid` layer tags.
- Current Rust drift is call-site selection: rebuild and blocker maps use movement/path layer, runtime movement inserts with `active_layer`, and collapse `DropIn` clears `on_bridge` without relayering the occupancy cache.
- This follows existing `movement_bridge.rs` architecture: `loco.layer` follows A* path layer, while `on_bridge` follows the gamemd bridge predicate.
- No new INI key drives this fix. Existing bridge INI keys are contextual (`DestroyableBridges`, `BridgeStrength`, `BridgeExplosions`, `C4Warhead`, `BridgeRepairHut`, `TooBigToFitUnderBridge`, `MovementZone`, `SpeedType`), but this plan adds no parser work.
- Remaining unknowns after grounding are deliberately out of scope: teleport/hover/jumpjet/aircraft/carryall writer families, low-bridge `TubeClass`, and an explicit `+0x124/+0x128` bitfield reservation model.

## Key Technical Decisions

- Add `GameEntity::occupancy_list_layer() -> Option<MovementLayer>` instead of changing `movement_layer_or_ground`: this keeps object-list selection separate from path/movement layer and prevents broad pathing side effects. **Confidence:** high
  - **Source:** `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`; live Ghidra `0047E8A0`; repo pattern `src/sim/game_entity.rs`
- Use a pure `projected_on_bridge(current, update)` helper near `BridgeStateUpdate`: movement code needs the post-transition list layer before `apply_pending_bridge_render_state` mutates the entity. **Confidence:** high
  - **Source:** `src/sim/movement/movement_bridge.rs`; design doc data flow; live Ghidra `005683C0`/`005687F0` caller order from report
- Keep `OccupancyGrid` storage unchanged: the parity gap is the layer source passed to existing `add`/`move_entity`, not the vector representation or insertion policy. **Confidence:** high
  - **Source:** `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`; `src/sim/occupancy.rs`
- Apply the same list-layer helper to `LayeredEntityBlockMap`: A* soft blockers must be keyed by the same selected object list that runtime occupancy uses. **Confidence:** high
  - **Source:** `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`; `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`; `src/sim/movement/bump_crush.rs`
- Preserve Air/Underground exclusion in the new helper for this scoped fix. **Confidence:** medium
  - **Source:** current Rust occupancy policy in `src/sim/occupancy.rs`; report notes non-drive writer inventory is only medium confidence

## Open Questions

### Resolved During Planning

- Should `MovementLayer` be split into path layer and object-list layer enums? No. The targeted fix is smaller and follows existing API shape; clear helper and variable names are sufficient for this patch.
- Should `movement_layer_or_ground` be changed globally? No. It is used by path/movement code and changing it would collapse a distinction that `movement_bridge.rs` intentionally introduced.
- Should low bridge movement use this machinery? No. Ghidra confirms low bridges use `TubeClass`/tube-index semantics, separate from high-bridge `AltObject`.
- Should Rust code be implemented during this turn? No. This is a write-plan deliverable only.

### Deferred to Separate Research Or Plans

- Non-drive bridge list writers for teleport, hover, jumpjet, aircraft, and carryall are excluded because the verified report only traced normal drive/walk/ship families fully.
- A future explicit occupation-bit model for `+0x124/+0x128` is excluded because current Rust tracks object-list occupants, not gamemd bitfields.
- Save/load exact within-cell linked-list order remains a separate cache-rebuild parity topic.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/game_entity.rs` | Add explicit object-list layer helper and tests; clarify existing layer helper comment. |
| Modify | `src/sim/movement/movement_bridge.rs` | Add pure projection helper for post-transition `on_bridge` state and tests. |
| Modify | `src/sim/occupancy.rs` | Rebuild cache from `occupancy_list_layer` instead of locomotor/path layer; add disagreement tests. |
| Modify | `src/sim/movement/bump_crush.rs` | Key `LayeredEntityBlockMap` from object-list layer; add soft-block disagreement tests. |
| Modify | `src/sim/movement/movement_occupancy.rs` | Ensure deferred runtime entry detection sees selected object-list occupants. |
| Modify | `src/sim/pathfinding/cell_entry.rs` | Keep split `object_list_layer` / `occupancy_bits_layer` checks coherent after occupancy layers become object-list layers. |
| Modify | `src/sim/movement/movement_step.rs` | Use projected post-transition `on_bridge` for normal cell-crossing occupancy insertion. |
| Modify | `src/sim/movement/movement_tick.rs` | Use projected post-transition `on_bridge` for drive-track occupancy insertion. |
| Modify | `src/sim/world/bridge_orchestrator.rs` | Relayer deck occupants from Bridge to Ground during `DropIn`; add collapse cache tests. |
| Modify | `src/sim/movement/movement_tests.rs` | Extend existing bridge timing tests to assert occupancy/list layer at ramp/body/ground transitions. |

## Interface Changes

- Add `GameEntity::occupancy_list_layer(&self) -> Option<MovementLayer>`.
  - Depends on existing `GameEntity::on_bridge`, `LocomotorState.layer`, and `MovementLayer`.
  - Used by `OccupancyGrid::rebuild` and `build_entity_block_sets`.
- Add `movement_bridge::projected_on_bridge(current: bool, update: BridgeStateUpdate) -> bool`.
  - Used by `movement_step.rs` and `movement_tick.rs`.
  - Does not mutate entity state and does not alter tick ordering outside occupancy insertion layer selection.
- No public crate API, serialized state, INI schema, deterministic hash field, or app-layer interface changes.

## Sim Checklist

- [ ] All math uses `fixed`-point or integer state; no new `f32`/`f64` in game logic.
- [ ] No new authoritative state is added, so deterministic hash coverage is unchanged.
- [ ] No dependencies on render/ui/sidebar/audio/net.
- [ ] Tick ordering impact is limited to choosing the occupancy add layer from projected `on_bridge` before `apply_pending_bridge_render_state`.
- [ ] `BTreeMap`/`EntityStore` iteration order remains unchanged.

## Risk Areas

- `movement_layer_or_ground` has a tempting name; the implementation must avoid using it for object-list membership after the new helper exists.
- Runtime cell-entry has split layer semantics: `object_list_layer` selects occupants to classify, while `occupancy_bits_layer` approximates gamemd bitfield presence. After this plan, `OccupancyGrid` layer tags represent object-list membership, so phase-1 checks must not miss a selected-list blocker by probing only the bitfield layer.
- Movement crossing currently moves occupancy before bridge-state resolution in `movement_step.rs`; reordering must preserve remove-before-add and infantry sub-cell reservation behavior.
- Drive-track crossing already resolves bridge state before occupancy movement but still passes `active_layer`; changing only that argument must not disturb drive-track progression.
- `DropIn` same-cell relayering must keep sub-cell and `CellListInsertion` semantics intact while clearing stale Bridge-layer entries.
- A* soft blocker maps affect path shape; tests must pin the layer-key disagreement cases so deck and under-bridge units do not cost/block each other through the wrong selected list.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `on_bridge` wins over `loco.layer` for object-list membership | Units on ramps can have path layer and object-list layer disagree; wrong layer makes bridge and under-bridge units block each other incorrectly | Unit tests for helper disagreement cases |
| Task 3 | Rebuild reconstructs the same list layer as runtime | Debug/load rebuilds must not move a ramp unit to the wrong list | `cargo test -p vera20k occupancy -- --nocapture` |
| Task 4 | Soft blockers are keyed by object-list layer | A* should not add movement costs from units on the other vertical layer | `cargo test -p vera20k bump_crush -- --nocapture` |
| Task 5 | Runtime entry detection sees selected object-list blockers | A bridge-list occupant must block bridge-list movement even when bitfield-layer context is Ground | `movement_occupancy` / `cell_entry` split-layer tests |
| Task 6 | Ramp-up/ramp-down runtime insertion uses projected `on_bridge` | Players can stack under/on-bridge occupants in the same 2D cell without cross-layer blockage | Bridge movement tests assert occupancy layer per tick |
| Task 8 | Collapse `DropIn` relayers deck occupants to ground | After bridge collapse, deck units must stop occupying/blocking the destroyed deck list and survive at ground level | Bridge orchestrator DropIn tests |

---

## Tasks

### Task 1: Add `GameEntity::occupancy_list_layer`

**Why:** This creates the central contract for gamemd `FirstObject`/`AltObject` selection before any call site changes.

**Files:**
- Modify: `src/sim/game_entity.rs`

**Pattern:** Existing helper methods on `GameEntity`; no new module.

**Step 1: Add the helper beside `movement_layer_or_ground`**

```rust
/// Object-list layer for occupancy/cache membership.
///
/// This mirrors gamemd's `ObjectClass+0x8C` / `OnBridge` selector for
/// `CellClass::FirstObject` versus `AltObject`. It is intentionally not the
/// same as locomotor/path layer; ramps can have `loco.layer` and `on_bridge`
/// disagree for a tick.
pub fn occupancy_list_layer(&self) -> Option<crate::sim::movement::locomotor::MovementLayer> {
    use crate::sim::movement::locomotor::MovementLayer;

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

**Step 2: Clarify the existing helper comment**

```rust
/// Runtime movement/path layer with Ground as the fallback.
///
/// This is not the object-list selector. Use `occupancy_list_layer` when
/// selecting gamemd `FirstObject` versus `AltObject` style occupancy.
pub fn movement_layer_or_ground(&self) -> crate::sim::movement::locomotor::MovementLayer {
```

**Step 3: Add focused tests in `game_entity.rs`**

Use the local test module and a small test locomotor constructor:

```rust
fn test_loco(layer: crate::sim::movement::locomotor::MovementLayer) -> crate::sim::movement::locomotor::LocomotorState {
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::movement::locomotor::{AirMovePhase, GroundMovePhase, LocomotorState};
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO};

    LocomotorState {
        kind: LocomotorKind::Drive,
        layer,
        phase: GroundMovePhase::Idle,
        air_phase: AirMovePhase::Landed,
        speed_multiplier: SIM_ONE,
        speed_fraction: SIM_ONE,
        fly_current_speed: SIM_ZERO,
        altitude: SIM_ZERO,
        target_altitude: SIM_ZERO,
        climb_rate: SIM_ZERO,
        jumpjet_speed: SIM_ZERO,
        jumpjet_wobbles: 0.0,
        jumpjet_accel: SIM_ZERO,
        jumpjet_current_speed: SIM_ZERO,
        jumpjet_deviation: 0,
        jumpjet_crash_speed: SIM_ZERO,
        jumpjet_turn_rate: 4,
        balloon_hover: false,
        hover_attack: false,
        speed_type: SpeedType::Track,
        movement_zone: MovementZone::Normal,
        rot: 0,
        override_state: None,
        air_progress: SIM_ZERO,
        infantry_wobble_phase: 0.0,
        subcell_dest: None,
    }
}

#[test]
fn occupancy_list_layer_uses_on_bridge_over_loco_bridge() {
    use crate::sim::movement::locomotor::MovementLayer;

    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
    e.on_bridge = false;
    e.locomotor = Some(test_loco(MovementLayer::Bridge));

    assert_eq!(e.occupancy_list_layer(), Some(MovementLayer::Ground));
}

#[test]
fn occupancy_list_layer_uses_on_bridge_over_loco_ground() {
    use crate::sim::movement::locomotor::MovementLayer;

    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
    e.on_bridge = true;
    e.locomotor = Some(test_loco(MovementLayer::Ground));

    assert_eq!(e.occupancy_list_layer(), Some(MovementLayer::Bridge));
}

#[test]
fn occupancy_list_layer_skips_air_and_underground() {
    use crate::sim::movement::locomotor::MovementLayer;

    let mut air = GameEntity::test_default(1, "ORCA", "Americans", 5, 5);
    air.on_bridge = true;
    air.locomotor = Some(test_loco(MovementLayer::Air));
    assert_eq!(air.occupancy_list_layer(), None);

    let mut tunnel = GameEntity::test_default(2, "DVIL", "Soviet", 5, 5);
    tunnel.on_bridge = true;
    tunnel.locomotor = Some(test_loco(MovementLayer::Underground));
    assert_eq!(tunnel.occupancy_list_layer(), None);
}
```

**Step 4: Verify**

Run: `cargo test -p vera20k game_entity -- --nocapture`

Expected: PASS.

**Step 5: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 2: Add `projected_on_bridge` helper

**Why:** Movement call sites need the post-transition object-list layer before the entity's `on_bridge` field is mutated.

**Files:**
- Modify: `src/sim/movement/movement_bridge.rs`

**Pattern:** Pure helper next to `BridgeStateUpdate`, like `compute_bridge_transition`.

**Step 1: Add the helper**

```rust
pub(super) fn projected_on_bridge(current: bool, update: BridgeStateUpdate) -> bool {
    match update {
        BridgeStateUpdate::Set(_) => true,
        BridgeStateUpdate::Clear => false,
        BridgeStateUpdate::Unchanged => current,
    }
}
```

**Step 2: Add tests in `movement_bridge.rs`**

```rust
#[test]
fn projected_on_bridge_applies_pending_update_without_mutation() {
    assert!(projected_on_bridge(false, BridgeStateUpdate::Set(4)));
    assert!(!projected_on_bridge(true, BridgeStateUpdate::Clear));
    assert!(projected_on_bridge(true, BridgeStateUpdate::Unchanged));
    assert!(!projected_on_bridge(false, BridgeStateUpdate::Unchanged));
}
```

**Step 3: Verify**

Run: `cargo test -p vera20k movement_bridge -- --nocapture`

Expected: PASS.

**Step 4: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 3: Rebuild occupancy from object-list layer

**Why:** Rebuild paths must reconstruct the same ground/bridge selected list as runtime movement and gamemd `AddContent`.

**Files:**
- Modify: `src/sim/occupancy.rs`

**Pattern:** Existing `OccupancyGrid::rebuild` scan over `EntityStore` in deterministic entity order.

**Step 1: Replace locomotor-layer derivation in `rebuild`**

```rust
let Some(layer) = entity.occupancy_list_layer() else {
    continue;
};
```

Remove the local `use crate::sim::movement::locomotor::MovementLayer;` inside `rebuild` if it becomes unused.

**Step 2: Preserve current insert behavior**

Keep these lines unchanged after the new layer selection:

```rust
let sub = if entity.category == EntityCategory::Infantry {
    entity.sub_cell
} else {
    None
};
let insertion = CellListInsertion::from_category(entity.category);
grid.add(rx, ry, sid, layer, sub, insertion);
```

**Step 3: Add tests in `occupancy.rs`**

Reuse the Task 1 test locomotor constructor shape locally or call an existing test helper if one is already in the same module.

```rust
#[test]
fn rebuild_uses_on_bridge_for_layer_even_when_loco_is_ground() {
    let mut entities = crate::sim::entity_store::EntityStore::new();
    let mut deck = crate::sim::game_entity::GameEntity::test_default(1, "HTNK", "Allies", 5, 5);
    deck.on_bridge = true;
    deck.locomotor = Some(test_loco(MovementLayer::Ground));
    entities.insert(deck);

    let grid = OccupancyGrid::rebuild(&entities);
    let cell = grid.get(5, 5).expect("cell occupancy");
    assert_eq!(cell.count_on(MovementLayer::Bridge), 1);
    assert_eq!(cell.count_on(MovementLayer::Ground), 0);
}

#[test]
fn rebuild_ignores_loco_bridge_without_on_bridge() {
    let mut entities = crate::sim::entity_store::EntityStore::new();
    let mut ramp = crate::sim::game_entity::GameEntity::test_default(1, "HTNK", "Allies", 5, 5);
    ramp.on_bridge = false;
    ramp.locomotor = Some(test_loco(MovementLayer::Bridge));
    entities.insert(ramp);

    let grid = OccupancyGrid::rebuild(&entities);
    let cell = grid.get(5, 5).expect("cell occupancy");
    assert_eq!(cell.count_on(MovementLayer::Ground), 1);
    assert_eq!(cell.count_on(MovementLayer::Bridge), 0);
}
```

**Step 4: Verify**

Run: `cargo test -p vera20k occupancy -- --nocapture`

Expected: PASS.

**Step 5: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 4: Key soft blockers by object-list layer

**Why:** Pathfinding cost prediction should see blockers from the selected gamemd object list, not from an entity's path layer while it is on a ramp.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs`

**Pattern:** Existing `build_entity_block_sets` loop and `LayeredEntityBlockMap::insert`.

**Step 1: Replace layer selection in `build_entity_block_sets`**

```rust
let Some(layer) = entity.occupancy_list_layer() else {
    continue;
};
```

Remove the following Air/Underground skip because the helper now owns it:

```rust
if matches!(layer, MovementLayer::Air | MovementLayer::Underground) {
    continue;
}
```

**Step 2: Keep structure behavior unchanged**

Structures must continue to use `ground_blocked` and footprint expansion in this task. Do not route building hard-block footprints through `bridge_blocked`; that is a separate bridge-building parity model.

**Step 3: Add tests in `bump_crush.rs`**

Use `build_entity_block_sets`, the local test entity helpers, and a simple `HouseAllianceMap`.

```rust
#[test]
fn entity_block_map_uses_ground_when_loco_bridge_but_not_on_bridge() {
    let mut entities = EntityStore::new();
    let mut blocker = vehicle(10, 5, 5);
    blocker.owner = crate::sim::intern::test_intern("Soviet");
    blocker.on_bridge = false;
    blocker.locomotor = Some(test_loco(MovementLayer::Bridge));
    entities.insert(blocker);

    let interner = crate::sim::intern::test_interner();
    let alliances = crate::map::houses::HouseAllianceMap::new();
    let (_, _, blocks) = build_entity_block_sets(
        &entities,
        "Americans",
        &alliances,
        &interner,
        None,
    );

    assert!(blocks.get(MovementLayer::Ground, &(5, 5)).is_some());
    assert!(blocks.get(MovementLayer::Bridge, &(5, 5)).is_none());
}

#[test]
fn entity_block_map_uses_bridge_when_on_bridge_but_loco_ground() {
    let mut entities = EntityStore::new();
    let mut blocker = vehicle(10, 5, 5);
    blocker.owner = crate::sim::intern::test_intern("Soviet");
    blocker.on_bridge = true;
    blocker.locomotor = Some(test_loco(MovementLayer::Ground));
    entities.insert(blocker);

    let interner = crate::sim::intern::test_interner();
    let alliances = crate::map::houses::HouseAllianceMap::new();
    let (_, _, blocks) = build_entity_block_sets(
        &entities,
        "Americans",
        &alliances,
        &interner,
        None,
    );

    assert!(blocks.get(MovementLayer::Bridge, &(5, 5)).is_some());
    assert!(blocks.get(MovementLayer::Ground, &(5, 5)).is_none());
}
```

If `LayeredEntityBlockMap` does not expose `get`, add a small test-only accessor instead of inspecting private fields from outside the type.

**Step 4: Verify**

Run: `cargo test -p vera20k bump_crush -- --nocapture`

Expected: PASS.

**Step 5: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 5: Align runtime cell-entry selected-list checks

**Why:** Once `OccupancyGrid` layer tags represent gamemd object-list membership, runtime entry detection must not decide that a selected-list blocker is absent by probing only `occupancy_bits_layer`.

**Files:**
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/pathfinding/cell_entry.rs`

**Pattern:** Existing split-layer model: `object_list_layer` controls occupant classification; `occupancy_bits_layer` controls bitfield/subcell-style availability checks.

**Step 1: Update vehicle deferred detection in `movement_occupancy.rs`**

Keep the infantry sub-cell test on `occupancy_bits_layer`, but make vehicle detection also check selected object-list occupants. Add a `current_object_list_layer: MovementLayer` parameter to `detect_deferred_cell_check` so the self-cell guard compares object-list membership to object-list membership, not to path `active_layer`.

```rust
let occupancy_bits_layer = layer_context.occupancy_bits_layer;
let object_list_layer = layer_context.object_list_layer;
let is_self_cell =
    (next_cell.0, next_cell.1, object_list_layer)
        == (current_cell.0, current_cell.1, current_object_list_layer);
if is_self_cell {
    return None;
}

let cell_occ = occupancy.get(next_cell.0, next_cell.1);
if mover_category == EntityCategory::Infantry {
    if bump_crush::allocate_sub_cell_with_reserved(cell_occ, occupancy_bits_layer, None).is_none()
        || cell_occ.is_some_and(|o| {
            o.has_blockers_on(object_list_layer) || o.infantry(object_list_layer).next().is_some()
        })
    {
        return Some(DeferredCellCheck::Infantry(next_cell, layer_context));
    }
} else if cell_occ.is_some_and(|o| {
    o.has_blockers_on(object_list_layer)
        || o.infantry(object_list_layer).next().is_some()
        || o.has_blockers_on(occupancy_bits_layer)
        || o.infantry(occupancy_bits_layer).next().is_some()
}) {
    return Some(DeferredCellCheck::Vehicle(next_cell, layer_context));
}
```

This preserves the current bitfield-layer approximation while guaranteeing selected-list occupants reach phase 2 classification.

At the `movement_step.rs` call site, pass the mover's current object-list layer from the snapshot:

```rust
let current_object_list_layer = if snap.on_bridge {
    MovementLayer::Bridge
} else {
    MovementLayer::Ground
};
```

Use this value for `detect_deferred_cell_check`. Keep `active_layer` available for path/movement recovery logic.

Task 6 replaces this one-shot value with the mutable per-loop `projected_on_bridge_state` once normal crossing projection exists. The important contract is that the self-cell guard receives the mover's current object-list layer, not its path layer.

**Step 2: Update `cell_entry::check_terrain_with_layers`**

For infantry, preserve sub-cell allocation on `occupancy_bits_layer` but return `NeedsBlockerCheck` if the selected object list has blockers:

```rust
if mover_category == EntityCategory::Infantry {
    let selected_list_blocked = occ.is_some_and(|o| {
        o.has_blockers_on(layers.object_list_layer)
            || o.infantry(layers.object_list_layer).next().is_some()
    });
    let sub =
        bump_crush::allocate_sub_cell_with_reserved(occ, layers.occupancy_bits_layer, None);
    if sub.is_some() && !selected_list_blocked {
        return TerrainCheckResult::Clear;
    }
    return TerrainCheckResult::NeedsBlockerCheck;
}
```

For vehicles, treat either layer as needing phase-2 classification:

```rust
match occ {
    None => TerrainCheckResult::Clear,
    Some(o)
        if o.is_empty_on(layers.object_list_layer)
            && o.is_empty_on(layers.occupancy_bits_layer) =>
    {
        TerrainCheckResult::Clear
    }
    Some(_) => TerrainCheckResult::NeedsBlockerCheck,
}
```

**Step 3: Add split-layer tests**

In `movement_occupancy.rs`, add a test where `object_list_layer=Bridge`, `occupancy_bits_layer=Ground`, and the cell has only a Bridge blocker:

```rust
#[test]
fn deferred_detection_uses_object_list_layer_for_selected_blockers() {
    let mut occ = OccupancyGrid::new();
    occ.add(
        5,
        5,
        10,
        MovementLayer::Bridge,
        None,
        CellListInsertion::PrependNonBuilding,
    );

    let check = detect_deferred_cell_check(
        EntityCategory::Unit,
        false,
        CanEnterLayerContext {
            terrain_layer: MovementLayer::Bridge,
            object_list_layer: MovementLayer::Bridge,
            occupancy_bits_layer: MovementLayer::Ground,
        },
        (5, 5),
        (4, 5),
        MovementLayer::Ground,
        MovementLayer::Bridge,
        &occ,
    );

    assert!(matches!(check, Some(DeferredCellCheck::Vehicle((5, 5), _))));
}
```

In `cell_entry.rs`, add the same split context against `check_terrain_with_layers` and assert `TerrainCheckResult::NeedsBlockerCheck`.

**Step 4: Verify**

Run:

```powershell
cargo test -p vera20k movement_occupancy -- --nocapture
cargo test -p vera20k cell_entry split_context -- --nocapture
```

Expected: PASS.

**Step 5: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 6: Use projected list layer in normal movement crossings

**Why:** Normal cell crossing currently inserts occupancy with `active_layer` before resolving the bridge predicate; this is the main ramp-tick list mismatch.

**Files:**
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Existing `resolve_cell_transition_bridge_state` plus pending `apply_pending_bridge_render_state` flow.

**Step 1: Track projected bridge state through the crossing loop**

Before the `loop` in `process_cell_crossings`, add:

```rust
let mut projected_on_bridge_state = snap.on_bridge;
```

This must be mutable because one tick can cross multiple cells. A fast Ramp -> Body -> Body tick must project the second crossing from the first crossing's projected `true`, not from the pre-loop snapshot.

**Step 2: Move bridge resolution before `occupancy.move_entity`**

Place this immediately after `apply_cell_transition_remainder` and before `occupancy.move_entity`:

```rust
let bridge_update = resolve_cell_transition_bridge_state(
    position,
    path_grid,
    (old_rx, old_ry),
    (nx, ny),
    next_layer,
);
projected_on_bridge_state =
    super::movement_bridge::projected_on_bridge(projected_on_bridge_state, bridge_update);
if !matches!(
    bridge_update,
    super::movement_bridge::BridgeStateUpdate::Unchanged
) {
    pending_bridge_update = bridge_update;
}
```

Then choose the occupancy layer from `projected_on_bridge_state`:

```rust
let occupancy_layer = if projected_on_bridge_state {
    MovementLayer::Bridge
} else {
    MovementLayer::Ground
};
```

**Step 3: Pass `occupancy_layer` to `move_entity`**

```rust
occupancy.move_entity(
    old_rx,
    old_ry,
    nx,
    ny,
    entity_id,
    occupancy_layer,
    *sub_cell,
    insertion,
);
```

**Step 4: Preserve path-layer state**

Keep this after occupancy movement:

```rust
active_layer = next_layer;
if let Some(loco) = locomotor {
    loco.layer = next_layer;
}
```

Do not mutate `on_bridge` in `movement_step.rs`; keep that in `apply_pending_bridge_render_state`.

When Task 5's `detect_deferred_cell_check` call is updated in this same function, pass the current object-list layer from `projected_on_bridge_state`:

```rust
let current_object_list_layer = if projected_on_bridge_state {
    MovementLayer::Bridge
} else {
    MovementLayer::Ground
};
```

This prevents later loop iterations from using the pre-loop `snap.on_bridge` after an earlier crossing projected `Set` or `Clear`.

**Step 5: Extend bridge timing tests**

In `src/sim/movement/movement_tests.rs`, add occupancy assertions to the existing bridge timing tests:

```rust
let cell = occupancy.get(2, 1).expect("destination occupancy");
assert_eq!(
    cell.count_on(MovementLayer::Bridge),
    1,
    "Ramp->Body inserts into bridge object list after on_bridge projects true"
);
assert_eq!(cell.count_on(MovementLayer::Ground), 0);
```

For the ramp-down test, after body to ramp:

```rust
let ramp_cell = occupancy.get(2, 1).expect("ramp occupancy");
assert_eq!(
    ramp_cell.count_on(MovementLayer::Bridge),
    1,
    "Body->Ramp keeps bridge object list while on_bridge remains true"
);
assert_eq!(ramp_cell.count_on(MovementLayer::Ground), 0);
```

After ramp to ground:

```rust
let ground_cell = occupancy.get(3, 1).expect("ground occupancy");
assert_eq!(ground_cell.count_on(MovementLayer::Ground), 1);
assert_eq!(ground_cell.count_on(MovementLayer::Bridge), 0);
```

For the no-lookahead test, after ground to ramp:

```rust
let ramp_cell = occupancy.get(2, 1).expect("ramp occupancy");
assert_eq!(
    ramp_cell.count_on(MovementLayer::Ground),
    1,
    "Ground->Ramp stays ground object list while on_bridge remains false"
);
assert_eq!(ramp_cell.count_on(MovementLayer::Bridge), 0);
```

Add one high-speed multi-crossing test where a mover crosses Ramp -> Body -> Body in one tick. Assert both final entity `on_bridge=true` and final occupancy layer Bridge. This test specifically guards against overwriting the first crossing's `Set(deck)` with a later `Unchanged`.

**Step 6: Verify**

Run: `cargo test -p vera20k movement_tests bridge -- --nocapture`

Expected: PASS.

**Step 7: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 7: Use projected list layer in drive-track crossings

**Why:** Drive-track movement already computes `BridgeStateUpdate` before occupancy movement, but it still inserts using `active_layer`.

**Files:**
- Modify: `src/sim/movement/movement_tick.rs`
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Existing drive-track branch in `tick_movement_with_grid`.

**Step 1: Compute an `occupancy_layer` variable before `occupancy.move_entity`**

Initialize before the optional `path_grid` block:

```rust
let mut occupancy_layer = if entity.on_bridge {
    MovementLayer::Bridge
} else {
    MovementLayer::Ground
};
```

Inside the `if let Some(pg) = path_grid` block, after `bridge_update` is set:

```rust
let new_on_bridge = super::movement_bridge::projected_on_bridge(
    entity.on_bridge,
    bridge_update,
);
occupancy_layer = if new_on_bridge {
    MovementLayer::Bridge
} else {
    MovementLayer::Ground
};
```

**Step 2: Pass `occupancy_layer` to drive-track `move_entity`**

```rust
occupancy.move_entity(
    old_rx,
    old_ry,
    nx,
    ny,
    entity_id,
    occupancy_layer,
    entity.sub_cell,
    CellListInsertion::from_category(entity.category),
);
```

**Step 3: Keep `active_layer` for locomotor/path state**

Do not replace `active_layer` in drive-track facing, reservation, or locomotor updates. Only the occupancy move call changes layer source.

**Step 4: Add a drive-track regression test**

Add a focused test that starts an entity on a body cell with `on_bridge=true`, drive-track movement to a ramp path layer `Ground`, and asserts the destination ramp occupancy remains Bridge until the subsequent ramp-to-ground crossing clears `on_bridge`.

```rust
assert_eq!(
    occupancy
        .get(ramp_rx, ramp_ry)
        .expect("ramp occupancy")
        .count_on(MovementLayer::Bridge),
    1
);
assert_eq!(
    occupancy
        .get(ramp_rx, ramp_ry)
        .expect("ramp occupancy")
        .count_on(MovementLayer::Ground),
    0
);
```

Use the same synthetic `PathGrid` shape and `make_drive_loco` helper already present in `movement_tests.rs`.

**Step 5: Verify**

Run: `cargo test -p vera20k movement_tests bridge -- --nocapture`

Expected: PASS.

**Step 6: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 8: Relayer occupancy during bridge collapse `DropIn`

**Why:** `DropIn` clears `on_bridge`; Rust must also move the occupancy cache entry from Bridge to Ground in the same cell.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Existing `drop_in_bridge_deck_entities` loop; existing `OccupancyGrid::move_entity` remove-plus-add semantics.

**Step 1: Capture relayer info without holding the entity borrow**

Inside `for id in to_snap`, use a temporary tuple:

```rust
let mut relayer = None;
if let Some(entity) = sim.entities.get_mut(id) {
    entity.bridge_occupancy = None;
    entity.on_bridge = false;
    entity.position.z = ground_level;
    entity.position.refresh_screen_coords();
    entity.movement_target = None;
    if let Some(ref mut loco) = entity.locomotor {
        loco.layer = MovementLayer::Ground;
        loco.phase = GroundMovePhase::Idle;
    }
    relayer = Some((
        entity.position.rx,
        entity.position.ry,
        entity.sub_cell,
        CellListInsertion::from_category(entity.category),
    ));
}
```

Add this import in the function:

```rust
use crate::sim::occupancy::CellListInsertion;
```

**Step 2: Move occupancy after the entity borrow ends**

```rust
if let Some((rx, ry, sub_cell, insertion)) = relayer {
    sim.occupancy.move_entity(
        rx,
        ry,
        rx,
        ry,
        id,
        MovementLayer::Ground,
        sub_cell,
        insertion,
    );
}
```

**Step 3: Extend existing DropIn test**

In `drop_in_snaps_deck_entity_to_ground_over_water_no_despawn`, seed occupancy before the call:

```rust
sim.occupancy.add(
    5,
    5,
    id,
    MovementLayer::Bridge,
    None,
    CellListInsertion::PrependNonBuilding,
);
```

After the call:

```rust
let cell = sim.occupancy.get(5, 5).expect("occupancy retained");
assert_eq!(cell.count_on(MovementLayer::Ground), 1);
assert_eq!(cell.count_on(MovementLayer::Bridge), 0);
```

**Step 4: Add a non-deck guard test**

Use the existing second collapse test shape at `bridge_orchestrator.rs` around the ground-layer occupant. Seed a ground occupant with `on_bridge=false` and a bridge-layer locomotor, call `drop_in_bridge_deck_entities`, and assert it remains ground-layer:

```rust
let cell = sim.occupancy.get(5, 5).expect("ground occupancy");
assert_eq!(cell.count_on(MovementLayer::Ground), 1);
assert_eq!(cell.count_on(MovementLayer::Bridge), 0);
```

**Step 5: Verify**

Run: `cargo test -p vera20k bridge_orchestrator drop_in -- --nocapture`

Expected: PASS.

**Step 6: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 9: Guard scoped call sites and non-drive exclusions

**Why:** The accepted scope is drive/walk/ship-style high-bridge movement and collapse. This task prevents accidental edits in deferred systems.

**Files:**
- Inspect: `src/sim/movement/teleport_movement.rs`
- Inspect: `src/sim/movement/air_movement.rs`
- Inspect: `src/sim/movement/tunnel_movement.rs`
- Inspect: `src/sim/movement/parachute_descent.rs`
- Inspect: `src/sim/movement/droppod_movement.rs`

**Pattern:** Read-only guard scan; no behavior change unless compilation requires import cleanup from earlier tasks.

**Step 1: Search for remaining direct layer-derived occupancy moves**

Run:

```powershell
rg -n "occupancy\\.(add|move_entity)\\(|movement_layer_or_ground\\(|\\.layer\\)" src/sim/movement src/sim/world src/sim/occupancy.rs src/sim/game_entity.rs
```

**Step 2: Classify remaining hits**

Expected classifications:

- Normal movement and drive-track hits should be updated by Tasks 6 and 7.
- `OccupancyGrid::rebuild` should be updated by Task 3.
- `build_entity_block_sets` should be updated by Task 4.
- Runtime `movement_occupancy` / `cell_entry` split-layer detection should be updated by Task 5.
- `teleport_movement`, `air_movement`, `tunnel_movement`, `parachute_descent`, and `droppod_movement` remain deferred unless they fail tests because of API changes.

**Step 3: Add a short code comment only if needed**

If a deferred file still visibly uses locomotor/path layer for occupancy, add one concise comment at the call site:

```rust
// Non-drive bridge OnBridge writers are intentionally outside the current
// high-bridge occupancy-list parity patch.
```

Use this only where it prevents a future mistaken cleanup; avoid comment churn.

**Step 4: Verify no broad refactor occurred**

Run:

```powershell
git diff --stat
git diff -- src/sim/movement/teleport_movement.rs src/sim/movement/air_movement.rs src/sim/movement/tunnel_movement.rs src/sim/movement/parachute_descent.rs src/sim/movement/droppod_movement.rs
```

Expected: no behavioral changes in deferred locomotor families.

**Step 5: Checkpoint**

Review the diff for this task. Commit only if the user explicitly asks for commits.

### Task 10: Full verification pass

**Why:** The change crosses occupancy rebuild, movement tick order, bridge collapse, and A* soft blockers, so targeted tests and a broader sim check are both required.

**Files:**
- No source edits expected.

**Pattern:** Existing project cargo test flow.

**Step 1: Run targeted tests**

```powershell
cargo test -p vera20k game_entity -- --nocapture
cargo test -p vera20k occupancy -- --nocapture
cargo test -p vera20k movement_bridge -- --nocapture
cargo test -p vera20k movement_occupancy -- --nocapture
cargo test -p vera20k cell_entry split_context -- --nocapture
cargo test -p vera20k bump_crush -- --nocapture
cargo test -p vera20k movement_tests bridge -- --nocapture
cargo test -p vera20k bridge_orchestrator drop_in -- --nocapture
```

Expected: all PASS.

**Step 2: Run broader regression**

```powershell
cargo test -p vera20k sim -- --nocapture
```

Expected: PASS, or only pre-existing unrelated failures with exact test names recorded.

**Step 3: Check for forbidden architecture drift**

```powershell
rg -n "crate::(render|ui|sidebar|audio|net)::" src/sim
rg -n "f32|f64" src/sim/game_entity.rs src/sim/occupancy.rs src/sim/movement/movement_bridge.rs src/sim/movement/movement_occupancy.rs src/sim/pathfinding/cell_entry.rs src/sim/movement/movement_step.rs src/sim/movement/movement_tick.rs src/sim/world/bridge_orchestrator.rs src/sim/movement/bump_crush.rs
```

Expected: no new sim dependency violations and no new floating-point game logic. Existing render-only `f32` fields in `LocomotorState` are not introduced by this plan.

**Step 4: Review diff for scope**

```powershell
git diff -- src/sim/game_entity.rs src/sim/occupancy.rs src/sim/movement/movement_bridge.rs src/sim/movement/movement_occupancy.rs src/sim/pathfinding/cell_entry.rs src/sim/movement/movement_step.rs src/sim/movement/movement_tick.rs src/sim/world/bridge_orchestrator.rs src/sim/movement/bump_crush.rs src/sim/movement/movement_tests.rs
```

Expected: only layer-source selection, projection helper, cache relayering, tests, and narrowly relevant comments.

**Step 5: Final checkpoint**

Record test results and remaining unrelated failures, if any. Commit only if the user explicitly asks for commits.

## Sources & References

- **Design doc:** `docs/plans/2026-05-16-bridge-occupancy-onbridge-list-selector-design.md`
- **Primary verified report:** `docs/research/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`
- **Supporting Ghidra reports:** `BRIDGE_SYSTEM.md`, `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`, `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`, `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`, `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`, `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`, `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`
- **Gap scans:** `docs/gap-scans/2026-05-15-disparity-scan-bridges-end-to-end.md`, `docs/gap-scans/2026-05-15-disparity-scan-pathfinding-parity.md`
- **Live Ghidra checks this planning pass:** `CellClass::AddContent @ 0047E8A0`, `ObjectClass::DropIn @ 005F4160`, `CellClass::BlowUpBridge @ 0047DD70`, `UnitClass::Can_Enter_Cell @ 0073F0A0`, `TechnoClass::EnterCell_AddToMultiCells @ 005683C0`, `TechnoClass::ExitCell_RemoveFromMultiCells @ 005687F0`
- **Relevant CellClass fields:** `+0xE4` ground object-list head, `+0xE8` bridge object-list head, `+0x124` ground occupation bits, `+0x128` bridge occupation bits, `+0x140` bridge flags
- **Related code:** `src/sim/game_entity.rs`, `src/sim/occupancy.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/pathfinding/core.rs`
- **Contextual INI keys:** `rulesmd.ini [General] DestroyableBridges=yes`, `BridgeStrength=1500`, `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`; `rulesmd.ini [CABHUT] BridgeRepairHut=yes`; unit `MovementZone`, `SpeedType`, and `TooBigToFitUnderBridge` entries remain parsed by existing systems and are not modified here.
