# Building Scatter Fix — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Stop refineries (and any building) from being issued movement commands when a harvester drives into the foundation footprint during the dock sequence.

**Architecture:** Two-layer fix in `sim/movement` and `sim/pathfinding`. (1) Filter Structure occupants out of the primary-blocker scan in `find_primary_blocker` when the mover has `bypass_grid=true` — so foundation cells appear `Clear` to a docking harvester. (2) Add a Structure early-return in `scatter_blocker` as a safety net before the RNG read, preserving determinism. No changes to render/ui/audio. Tick ordering and state hash unaffected.

**Design Doc:** [docs/plans/2026-04-28-building-scatter-fix-design.md](docs/plans/2026-04-28-building-scatter-fix-design.md)

---

## Grounding Summary

- **ra2-rust-game-docs/:**
  - [SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md](docs/research/SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md) — verifies vtable+0x174 Scatter slot exists ONLY for `UnitClass` (0x743A50) and `InfantryClass` (0x51D0D0). No `BuildingClass::Scatter`. `CellClass::Scatter_Objects` filters via `FilterToTechno` which rejects RTTI 6 (Building).
  - [CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md](docs/research/CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md) — buildings mark cells with bit 0x40 ("Building present"), units with bit 0x20 ("Vehicle/unit"). Friendly-blocker checks only consider 0x20.
  - [MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md) — harvester dock drive is choreographed via radio commands, not normal pathfinding.
- **Ghidra verification:** Vtable slot table verified directly. RTTI filter verified via decompilation of `FilterToTechno`. Confidence high — these are mechanical lookups, not behavioral inferences.
- **Repo pattern mirrored:** `classify_occupied_cell` ([cell_entry.rs:148](src/sim/pathfinding/cell_entry.rs#L148)) already takes mover-specific params (`mover_zone`, `mover_omni_crusher`, `mover_locomotor`). `mover_bypass_grid` joins that group identically. `MoverSnapshot` ([mod.rs:122](src/sim/movement/mod.rs#L122)) already carries analogous mover flags (`omni_crusher`, `too_big_to_fit_under_bridge`); `bypass_grid` follows the same pattern. `MovementTarget.bypass_grid` field already exists at [components.rs:263](src/sim/components.rs#L263) — no new field on the entity, only on the snapshot.
- **INI keys driving behavior:** None. This is a movement/occupancy fix, not data-driven.
- **Still unknown:** None. Open question from design doc (Option 1 vs Option 2 for the `find_primary_blocker → None` case) is resolved below.

## Key Technical Decisions

- **Filter at `find_primary_blocker`, not at the consumer.** The "is this cell a blocker for this mover" decision belongs in cell_entry, where `mover_zone` and `mover_locomotor` already gate similar decisions. — **Confidence:** high. **Source:** repo pattern, design doc Approach A.
- **When `find_primary_blocker` returns None and `mover_bypass_grid` is true, `classify_occupied_cell` returns `Clear` (not `Impassable`).** Resolves the design doc's Open Question. The current None-branch comment says "shouldn't happen if Phase 1 said NeedsBlockerCheck" — but with bypass_grid filtering, it CAN happen (only structures present, all filtered out). For that case the cell is clear from the mover's perspective, by definition of `bypass_grid`. — **Confidence:** high. **Source:** design intent.
- **Add `bypass_grid: bool` to `MoverSnapshot`** rather than re-fetching from `entities.get(entity_id)` at the call site. Matches existing pattern (snap carries mover params). — **Confidence:** high. **Source:** [movement_tick.rs:54](src/sim/movement/movement_tick.rs#L54) `snapshot_mover`.
- **`scatter_blocker` Structure guard placed BEFORE the `rng.next_range_u32(8)` call.** Preserves RNG consumption order for every legitimate (non-Structure) scatter case. — **Confidence:** high. **Source:** determinism rule from CLAUDE.md.
- **Do NOT fix the unrelated `bypass_grid: false` reset during segment repath at [movement_tick.rs:226](src/sim/movement/movement_tick.rs#L226).** This is out of scope. Dock-drive paths are 2 cells with `final_goal: None` so they hit the "Finished" branch, never segment repath. Latent issue unrelated to this fix.

## Open Questions

### Resolved During Planning

- **What does `classify_occupied_cell` return when `find_primary_blocker` returns None due to bypass_grid filtering?** → `Clear`. See Key Technical Decisions.
- **Where does `MoverSnapshot.bypass_grid` get populated?** → In `snapshot_mover` ([movement_tick.rs:54](src/sim/movement/movement_tick.rs#L54)) via `e.movement_target.as_ref().map(|mt| mt.bypass_grid).unwrap_or(false)`.
- **Does this break the existing `harvester_undocks_through_foundation_to_outside_ore` test?** → No. That test uses an empty `OccupancyGrid` and never hits this code path. Verified via Read of [miner_tests.rs:1547-1565](src/sim/miner/miner_tests.rs#L1547-L1565).

### Deferred to Implementation

- None. All decisions are made.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/movement/mod.rs:122-132` | Add `bypass_grid: bool` field to `MoverSnapshot` |
| Modify | `src/sim/movement/movement_tick.rs:54-71` | Populate `bypass_grid` in `snapshot_mover` |
| Modify | `src/sim/pathfinding/cell_entry.rs:148-193` | Add `mover_bypass_grid` param to `classify_occupied_cell`; treat None-blocker as `Clear` when bypass set |
| Modify | `src/sim/pathfinding/cell_entry.rs:197-212` | Add `mover_bypass_grid` and `entities` params to `find_primary_blocker`; filter Structure occupants when bypass set |
| Modify | `src/sim/movement/movement_occupancy.rs:156-168` | Pass `snap.bypass_grid` to `classify_occupied_cell` call |
| Modify | `src/sim/movement/bump_crush.rs:472-490` | Add `EntityCategory::Structure` early-return at top of `scatter_blocker`, before RNG read |
| Modify | `src/sim/movement/bump_crush.rs` (tests) | Add `scatter_blocker_skips_structure` |
| Modify | `src/sim/pathfinding/cell_entry.rs` (tests) | Add `find_primary_blocker_skips_structure_with_bypass_grid` |
| Modify | `src/sim/miner/miner_tests.rs` | Add `harvester_drives_into_refinery_foundation_without_bumping_it` |

## Interface Changes

- **`classify_occupied_cell`** gains `mover_bypass_grid: bool`. Single call site in `handle_deferred_occupancy` updated in this plan.
- **`find_primary_blocker`** gains `mover_bypass_grid: bool` and `entities: &EntityStore`. Private to `cell_entry.rs`; only called from `classify_occupied_cell`.
- **`MoverSnapshot`** gains `bypass_grid: bool` field. Built only in `snapshot_mover` ([movement_tick.rs:54](src/sim/movement/movement_tick.rs#L54)). All readers go through `snap.bypass_grid`; existing readers unaffected.
- **`scatter_blocker`** signature unchanged; behavior change only.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 in game logic. (No new math added.)
- [x] New state included in deterministic state hash. (No new entity state. `MoverSnapshot` is per-tick scratch, not persistent.)
- [x] No dependencies on render/ui/sidebar/audio/net. (Changes confined to `sim/movement/` and `sim/pathfinding/`.)
- [x] Tick ordering impact noted. (None.)
- [x] BTreeMap iteration order considered. (`occ.blockers(layer)` returns blockers in occupancy-add order; the Structure filter is order-independent.)

## Risk Areas

- **Determinism (highest priority).** RNG consumption in `scatter_blocker` must remain identical for all non-Structure blockers. Mitigated by placing the Structure guard before `rng.next_range_u32(8)`. Regression covered by Task 4's unit test (asserts no RNG consumed for Structure blocker).
- **`find_primary_blocker` signature break.** Private function with one caller. Mechanical update.
- **Existing tests in `bump_crush.rs::tests`** — three `scatter_blocker_*` tests use `vehicle()` helpers that build Unit-category entities. The new Structure guard short-circuits before the RNG read, but only when category == Structure. Existing Unit tests still consume RNG identically. Mitigated by inspection — no shared state changes.
- **`harvester_undocks_through_foundation_to_outside_ore` test** uses empty `OccupancyGrid`, so doesn't hit this code path either before or after. Will remain green.
- **Foundation cell occupancy registration.** The bug only manifests in real games where the world spawn registers foundation cells. The new integration test (Task 5) explicitly populates the foundation in `OccupancyGrid` to reproduce.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Foundation cells appear `Clear` to a docking harvester | Player-visible: harvester drives smoothly into refinery instead of stalling, repathing, or moving the building. gamemd does this via choreographed radio drive that bypasses normal blocker check. | Unit test (Task 4) + integration test (Task 5). In-game observation: dock a harvester, refinery stays put, harvester reaches pad in normal time. |
| Task 2 | Buildings never receive movement commands via scatter | Player-visible: refinery (or any building) cannot drift on the map. gamemd: no `BuildingClass::Scatter`, `FilterToTechno` rejects RTTI 6. | Unit test asserting `scatter_blocker(structure_id)` returns false and attaches no `movement_target`. |

---

## Tasks

### Task 1: Wire `bypass_grid` through `MoverSnapshot` → `classify_occupied_cell` → `find_primary_blocker`

**Why:** This is the core fix. All five sub-changes must land together to keep the build green (signature changes + call sites). One commit, one logical change.

**Files:**
- Modify: `src/sim/movement/mod.rs:122-132`
- Modify: `src/sim/movement/movement_tick.rs:54-71`
- Modify: `src/sim/pathfinding/cell_entry.rs:148-212`
- Modify: `src/sim/movement/movement_occupancy.rs:156-168`

**Pattern:** Mirrors how `omni_crusher` and other mover flags flow: stored on `MoverSnapshot`, populated in `snapshot_mover`, passed into `classify_occupied_cell`. New `entities: &EntityStore` param on `find_primary_blocker` matches the param already taken by `classify_occupied_cell` (consistent at module level).

**Step 1: Add `bypass_grid` field to `MoverSnapshot`**

Open [src/sim/movement/mod.rs](src/sim/movement/mod.rs) at line 122. The struct currently ends with `rot: i32,` on line 131. Add `bypass_grid` as the last field:

```rust
pub(super) struct MoverSnapshot {
    pub category: EntityCategory,
    pub speed_type: Option<SpeedType>,
    pub movement_zone: MovementZone,
    pub omni_crusher: bool,
    pub owner: InternedId,
    pub too_big_to_fit_under_bridge: bool,
    pub on_bridge: bool,
    pub locomotor: Option<locomotor::LocomotorState>,
    pub rot: i32,
    /// Mover's `MovementTarget.bypass_grid` flag — when true, structure
    /// occupants are skipped during the foundation-cross occupancy check
    /// (matches gamemd's harvester dock drive: buildings are not scatter
    /// targets, FilterToTechno rejects RTTI 6).
    pub bypass_grid: bool,
}
```

**Step 2: Populate `bypass_grid` in `snapshot_mover`**

Open [src/sim/movement/movement_tick.rs](src/sim/movement/movement_tick.rs) at line 54. Update the `MoverSnapshot` struct literal:

```rust
fn snapshot_mover(entities: &EntityStore, entity_id: u64) -> Option<MoverSnapshot> {
    let e = entities.get(entity_id)?;
    Some(MoverSnapshot {
        category: e.category,
        speed_type: e.locomotor.as_ref().map(|l| l.speed_type),
        movement_zone: e
            .locomotor
            .as_ref()
            .map(|l| l.movement_zone)
            .unwrap_or(MovementZone::Normal),
        omni_crusher: e.omni_crusher,
        owner: e.owner,
        too_big_to_fit_under_bridge: e.too_big_to_fit_under_bridge,
        on_bridge: e.on_bridge,
        locomotor: e.locomotor.clone(),
        rot: e.locomotor.as_ref().map(|l| l.rot).unwrap_or(0),
        bypass_grid: e
            .movement_target
            .as_ref()
            .map(|mt| mt.bypass_grid)
            .unwrap_or(false),
    })
}
```

**Step 3: Update `find_primary_blocker` signature and body**

Open [src/sim/pathfinding/cell_entry.rs](src/sim/pathfinding/cell_entry.rs) at line 197. Replace the function:

```rust
/// Find the primary blocker entity in a cell (first vehicle/structure, or first
/// non-self infantry).
///
/// When `mover_bypass_grid` is true, occupants whose category is `Structure` are
/// skipped — this lets the harvester dock drive treat foundation cells as clear,
/// matching gamemd (no `BuildingClass::Scatter`, `FilterToTechno` rejects RTTI 6).
fn find_primary_blocker(
    target: (u16, u16),
    layer: MovementLayer,
    mover_id: u64,
    mover_bypass_grid: bool,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
) -> Option<u64> {
    let occ = occupancy.get(target.0, target.1)?;
    // Prefer vehicle/structure blockers over infantry, but skip structures
    // when the mover has bypass_grid set (e.g. a harvester driving into a
    // refinery's foundation footprint).
    if let Some(bid) = occ.blockers(layer).find(|&bid| {
        if !mover_bypass_grid {
            return true;
        }
        // Skip structures; keep units/aircraft.
        entities
            .get(bid)
            .map(|e| e.category != EntityCategory::Structure)
            .unwrap_or(true)
    }) {
        return Some(bid);
    }
    // Fall back to first non-self infantry.
    occ.infantry(layer)
        .find(|&(id, _)| id != mover_id)
        .map(|(id, _)| id)
}
```

**Step 4: Update `classify_occupied_cell` signature and None-branch**

In the same file, [cell_entry.rs:148-193](src/sim/pathfinding/cell_entry.rs#L148-L193):

```rust
pub fn classify_occupied_cell(
    target: (u16, u16),
    target_layer: MovementLayer,
    mover_id: u64,
    mover_zone: MovementZone,
    mover_omni_crusher: bool,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
) -> CellEntryResult {
    // --- Crush check ---
    let victims = bump_crush::collect_crush_victims(
        target,
        occupancy,
        target_layer,
        mover_zone,
        mover_omni_crusher,
        entities,
    );
    if !victims.is_empty()
        && bump_crush::cell_passable_after_crush(
            target,
            occupancy,
            target_layer,
            mover_zone,
            mover_omni_crusher,
            entities,
        )
    {
        return apply_overrides(CellEntryResult::Crushable { victims }, mover_locomotor);
    }

    // --- Find primary blocker ---
    let blocker_id = find_primary_blocker(
        target,
        target_layer,
        mover_id,
        mover_bypass_grid,
        occupancy,
        entities,
    );
    let Some(bid) = blocker_id else {
        // No identifiable blocker. With bypass_grid, this means the cell
        // contained only structures that we're permitted to drive through —
        // treat as Clear. Without bypass_grid, this is unexpected (Phase 1
        // would have returned Clear if the cell were truly empty).
        if mover_bypass_grid {
            return apply_overrides(CellEntryResult::Clear, mover_locomotor);
        }
        return apply_overrides(CellEntryResult::Impassable, mover_locomotor);
    };

    // --- Classify blocker ---
    let result = classify_blocker(bid, mover_owner, entities, alliances, interner);
    apply_overrides(result, mover_locomotor)
}
```

**Step 5: Update the call site in `handle_deferred_occupancy`**

Open [src/sim/movement/movement_occupancy.rs](src/sim/movement/movement_occupancy.rs) at line 156. Add `snap.bypass_grid` to the call:

```rust
    let entry_result = cell_entry::classify_occupied_cell(
        (nx, ny),
        next_layer,
        entity_id,
        snap.movement_zone,
        snap.omni_crusher,
        interner.resolve(snap.owner),
        mover_loco_kind,
        snap.bypass_grid,
        occupancy,
        entities,
        alliances,
        interner,
    );
```

**Step 6: Verify compile**

Run: `cargo check`
Expected: clean compile, no errors.

**Step 7: Commit**

Run:
```
git add src/sim/movement/mod.rs src/sim/movement/movement_tick.rs \
        src/sim/pathfinding/cell_entry.rs src/sim/movement/movement_occupancy.rs
git commit -m "movement: bypass_grid lets movers skip Structure occupancy checks

When a mover has MovementTarget.bypass_grid set (currently the harvester
dock drive into a refinery foundation), Structure occupants are filtered
out of find_primary_blocker. If only structures occupy the cell, the
mover sees it as Clear instead of OccupiedFriendly. This matches gamemd:
no BuildingClass::Scatter, FilterToTechno rejects RTTI 6 (Building),
buildings mark cells with bit 0x40 not the unit-bit 0x20."
```

---

### Task 2: Add `EntityCategory::Structure` safety net to `scatter_blocker`

**Why:** Defense-in-depth. Even if some future cell-entry path produces `OccupiedFriendly` with a Structure blocker_id, the building still cannot be moved. One-line guard, no functional change for legitimate scatter cases. Must precede the RNG read for determinism.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs:472-490`

**Pattern:** Same shape as the existing `blocker.movement_target.is_some()` early return. Adds one more reason to bail.

**Step 1: Add the Structure guard**

Open [src/sim/movement/bump_crush.rs](src/sim/movement/bump_crush.rs) at line 480. The function currently looks like:

```rust
pub fn scatter_blocker(
    entities: &mut EntityStore,
    blocker_id: u64,
    path_grid: Option<&PathGrid>,
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    rng: &mut SimRng,
) -> bool {
    // Read blocker properties (immutable borrow).
    let Some(blocker) = entities.get(blocker_id) else {
        return false;
    };
    // Don't scatter a blocker that's already moving.
    if blocker.movement_target.is_some() {
        return false;
    }
```

Add the Structure check between those two early returns:

```rust
pub fn scatter_blocker(
    entities: &mut EntityStore,
    blocker_id: u64,
    path_grid: Option<&PathGrid>,
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    rng: &mut SimRng,
) -> bool {
    // Read blocker properties (immutable borrow).
    let Some(blocker) = entities.get(blocker_id) else {
        return false;
    };
    // Buildings are immutable obstacles — gamemd has no BuildingClass::Scatter
    // and FilterToTechno rejects RTTI 6 before per-class dispatch. Bail before
    // the RNG read so determinism is preserved for all legitimate scatter cases.
    if blocker.category == EntityCategory::Structure {
        return false;
    }
    // Don't scatter a blocker that's already moving.
    if blocker.movement_target.is_some() {
        return false;
    }
```

**Step 2: Verify the import is in scope**

`EntityCategory` should already be imported at the top of the file. Confirm at line 18:
```rust
use crate::map::entities::EntityCategory;
```
If missing, add it. (At time of writing, it's present — Grep result line 18.)

**Step 3: Verify compile**

Run: `cargo check`
Expected: clean compile.

**Step 4: Commit**

```
git add src/sim/movement/bump_crush.rs
git commit -m "bump_crush: refuse to scatter Structure blockers

Safety net for the bypass_grid filter — even if some future cell-entry
path produces OccupiedFriendly with a Structure blocker_id, the building
cannot receive a movement command. Placed before the RNG read so RNG
consumption order is unchanged for every legitimate scatter case."
```

---

### Task 3: Unit test for `scatter_blocker` Structure guard

**Why:** Pin the safety-net behavior. Asserts a Structure blocker returns `false`, no `movement_target`, and (implicitly via test execution path) RNG was not consumed.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs` (existing `mod tests`)

**Pattern:** Mirror the existing `test_scatter_blocker_issues_movement` at [bump_crush.rs:759](src/sim/movement/bump_crush.rs#L759). Use a `Structure`-category entity instead of a `vehicle()`.

**Step 1: Add the test**

Open [src/sim/movement/bump_crush.rs](src/sim/movement/bump_crush.rs) at the `tests` module (search `// -- scatter_blocker tests --` near line 756). Add this test after the existing scatter_blocker tests:

```rust
    fn structure(id: u64, rx: u16, ry: u16) -> GameEntity {
        let mut e = GameEntity::test_default(id, "GAREFN", "Allies", rx, ry);
        e.category = EntityCategory::Structure;
        e.crushable = false;
        e
    }

    #[test]
    fn test_scatter_blocker_skips_structure() {
        let grid = PathGrid::new(10, 10);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(42);

        let mut store = EntityStore::new();
        store.insert(structure(100, 5, 5));

        let result = scatter_blocker(
            &mut store,
            100,
            Some(&grid),
            &occupancy,
            MovementLayer::Ground,
            &mut rng,
        );

        assert!(
            !result,
            "scatter_blocker must refuse Structure blockers (gamemd parity: \
             no BuildingClass::Scatter)"
        );

        // Structure must not have been issued any movement.
        let e = store.get(100).expect("structure still alive");
        assert!(
            e.movement_target.is_none(),
            "Structure must not receive a movement_target from scatter"
        );

        // RNG must NOT have been consumed (determinism: a fresh rng with seed 42
        // gives the same first u32 as one that hasn't been touched).
        let mut control_rng = SimRng::new(42);
        assert_eq!(
            rng.next_range_u32(8),
            control_rng.next_range_u32(8),
            "scatter_blocker must not consume RNG when bailing on a Structure blocker"
        );
    }
```

**Step 2: Run the test**

Run: `cargo test test_scatter_blocker_skips_structure -- --nocapture`
Expected: PASS.

**Step 3: Run all bump_crush tests to confirm nothing else broke**

Run: `cargo test --lib bump_crush`
Expected: all existing tests still PASS.

**Step 4: Commit**

```
git add src/sim/movement/bump_crush.rs
git commit -m "bump_crush_tests: pin Structure scatter-rejection (RNG-preserving)"
```

---

### Task 4: Unit test for `find_primary_blocker` Structure filter

**Why:** Pin the cell_entry filter. Asserts Structure occupants are skipped when `mover_bypass_grid=true`, and present when false (regression check that the filter doesn't fire unconditionally).

**Files:**
- Modify: `src/sim/pathfinding/cell_entry.rs` (existing `mod tests`)

**Pattern:** New test in the existing tests module ([cell_entry.rs:252](src/sim/pathfinding/cell_entry.rs#L252)). Tests `find_primary_blocker` directly to avoid building an `alliances` map and `StringInterner` for `classify_occupied_cell`.

**Step 1: Add the test**

Open [src/sim/pathfinding/cell_entry.rs](src/sim/pathfinding/cell_entry.rs) at the bottom of the `mod tests` block (after `test_non_jumpjet_no_override` ending around line 357). Add:

```rust
    #[test]
    fn find_primary_blocker_skips_structure_with_bypass_grid() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::game_entity::GameEntity;

        // Cell occupancy: a Structure (refinery) at (5, 5).
        let mut occ = OccupancyGrid::new();
        occ.add(5, 5, 100, MovementLayer::Ground, None);

        // EntityStore with the structure entity.
        let mut entities = EntityStore::new();
        let mut refinery = GameEntity::test_default(100, "GAREFN", "Allies", 5, 5);
        refinery.category = EntityCategory::Structure;
        entities.insert(refinery);

        // With bypass_grid=true: structure is filtered, no other occupants → None.
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42, // mover_id
            true, // mover_bypass_grid
            &occ,
            &entities,
        );
        assert_eq!(
            result, None,
            "with bypass_grid=true, Structure occupants must be filtered out"
        );

        // With bypass_grid=false: structure is the primary blocker → Some(100).
        let result = find_primary_blocker(
            (5, 5),
            MovementLayer::Ground,
            42,
            false, // mover_bypass_grid
            &occ,
            &entities,
        );
        assert_eq!(
            result,
            Some(100),
            "with bypass_grid=false, Structure must still be picked as blocker (regression)"
        );
    }
```

**Step 2: Run the test**

Run: `cargo test find_primary_blocker_skips_structure_with_bypass_grid -- --nocapture`
Expected: PASS.

**Step 3: Run the full cell_entry test suite to confirm regression**

Run: `cargo test --lib cell_entry`
Expected: all existing tests still PASS.

**Step 4: Commit**

```
git add src/sim/pathfinding/cell_entry.rs
git commit -m "cell_entry_tests: pin bypass_grid Structure filter in find_primary_blocker"
```

---

### Task 5: Integration test — harvester drives into refinery foundation without bumping it

**Why:** End-to-end pin for the bug. Reproduces the original symptom (refinery in OccupancyGrid, harvester drives in) and asserts the building doesn't move and doesn't get a `movement_target`. This test would FAIL on the current main without Tasks 1+2.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs`

**Pattern:** Mirrors the structure of `harvester_undocks_through_foundation_to_outside_ore` at [miner_tests.rs:1506](src/sim/miner/miner_tests.rs#L1506). Difference: explicitly registers foundation cells in `OccupancyGrid` (the existing test uses an empty grid, which is why it didn't catch this bug).

**Step 1: Add the test**

Open [src/sim/miner/miner_tests.rs](src/sim/miner/miner_tests.rs). Append after the last existing test in the file:

```rust
/// End-to-end pin for the foundation-bump bug. Places a refinery at (10, 10)
/// with its foundation cells registered in OccupancyGrid (the real-game
/// configuration), then drives a harvester into the pad. Asserts the refinery's
/// position is unchanged and it never receives a movement_target — i.e. the
/// bypass_grid filter prevents the building from being treated as a scatter
/// candidate when the harvester crosses into a foundation cell.
///
/// Without the bypass_grid Structure filter (Task 1) and the scatter_blocker
/// Structure guard (Task 2), this test would FAIL: scatter_blocker would issue
/// a direct move to the refinery and walk it to an adjacent cell.
#[test]
fn harvester_drives_into_refinery_foundation_without_bumping_it() {
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::occupancy::OccupancyGrid;
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::rng::SimRng;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    // 4x3 GAREFN at (10, 10) — foundation occupies (10..=13, 10..=12).
    // spawn_refinery returns (); EntityStore is keyed by stable_id, so we use
    // the sid we passed in (100) as the entity_id directly. Mirrors the
    // existing harvester_undocks_through_foundation_to_outside_ore pattern.
    spawn_refinery(&mut sim, 100, 10, 10);
    let refinery_id: u64 = 100;
    // Capture initial position fields. Position is Clone but not Copy, so we
    // can't `let p = entity.position` through a borrow — read individual
    // fields into primitives instead.
    let (rx_before, ry_before, sub_x_before, sub_y_before) = {
        let r = sim.entities.get(refinery_id).expect("refinery just spawned");
        (r.position.rx, r.position.ry, r.position.sub_x, r.position.sub_y)
    };

    // Register foundation cells in OccupancyGrid (the real-game configuration —
    // this is what the existing undock test omits, which is why it didn't catch
    // the bump bug).
    let mut occupancy = OccupancyGrid::new();
    for ry in 10u16..=12 {
        for rx in 10u16..=13 {
            occupancy.add(rx, ry, refinery_id, MovementLayer::Ground, None);
        }
    }

    let mut path_grid = PathGrid::new(32, 32);
    path_grid.block_building_footprint(10, 10, "4x3");

    // Harvester at queue cell (14, 11), state=Dock, dock_phase=RotateToPad,
    // cargo full so unloading would happen. Reservation already held.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 14, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("harvester entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::RotateToPad;
        miner.reserved_refinery = Some(100);
        // Cargo doesn't need to be full for this test — RotateToPad → EnterPad
        // is enough to trigger the foundation cell crossing that exposed the bug.
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);

    let alliances = HouseAllianceMap::new();
    let terrain_costs = BTreeMap::new();
    let mut rng = SimRng::new(0);

    // Tick enough for: rotate (~16 ticks for 90deg at HARVESTER_BODY_ROT) +
    // drive 1 cell west onto the pad. 60 ticks gives plenty of slack.
    for _ in 0..60 {
        crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
        crate::sim::movement::tick_movement_with_grid(
            &mut sim.entities,
            Some(&path_grid),
            &terrain_costs,
            &alliances,
            &mut occupancy,
            &mut rng,
            67,
            sim.tick,
            &sim.interner,
        );
        sim.tick += 1;
    }

    let refinery = sim.entities.get(refinery_id).expect("refinery still alive");

    // (1) Refinery position is exactly unchanged.
    assert_eq!(
        refinery.position.rx, rx_before,
        "refinery rx must not change when harvester docks; got rx={}",
        refinery.position.rx,
    );
    assert_eq!(
        refinery.position.ry, ry_before,
        "refinery ry must not change when harvester docks; got ry={}",
        refinery.position.ry,
    );
    assert_eq!(
        refinery.position.sub_x, sub_x_before,
        "refinery sub_x must not change",
    );
    assert_eq!(
        refinery.position.sub_y, sub_y_before,
        "refinery sub_y must not change",
    );

    // (2) Refinery never received a movement_target.
    assert!(
        refinery.movement_target.is_none(),
        "refinery must not have a movement_target — buildings cannot scatter",
    );

    // (3) Sanity: the harvester actually progressed (rotated or moved). If it
    // didn't, the test isn't exercising the foundation crossing.
    let harvester = sim.entities.get(miner_id).expect("harvester still alive");
    let progressed = harvester.position.rx != 14
        || harvester.position.ry != 11
        || harvester
            .miner
            .as_ref()
            .map(|m| m.dock_phase != RefineryDockPhase::RotateToPad)
            .unwrap_or(false);
    assert!(
        progressed,
        "test setup error: harvester did not progress past initial state — \
         pos=({},{}) phase={:?}",
        harvester.position.rx,
        harvester.position.ry,
        harvester.miner.as_ref().map(|m| m.dock_phase),
    );
}
```

**Step 2: Run the new test**

Run: `cargo test harvester_drives_into_refinery_foundation_without_bumping_it -- --nocapture`
Expected: PASS.

**Step 3: Run the full miner test suite to confirm no regression**

Run: `cargo test --lib miner`
Expected: all existing miner tests still PASS, including `harvester_undocks_through_foundation_to_outside_ore`.

**Step 4: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: pin no-foundation-bump on harvester dock entry

Reproduces the refinery-jumps-when-harvester-docks bug end-to-end:
foundation cells registered in OccupancyGrid, harvester drives onto pad,
asserts refinery position unchanged and no movement_target. The existing
undock test uses an empty OccupancyGrid so didn't catch this."
```

---

### Task 6: Full regression sweep + verification against gamemd

**Why:** Final safety check that no other test in the workspace broke from the signature changes, and confirm the fix matches gamemd's observable behavior.

**Files:** None modified. Run-only.

**Step 1: Run the full test suite**

Run: `cargo test`
Expected: all tests PASS, no new failures.

If any test fails:
- If it's a downstream caller of `classify_occupied_cell` — investigate; unexpected since the only call site is `handle_deferred_occupancy`.
- If it's an existing scatter test — investigate; existing tests use Unit-category entities so the new Structure guard should not fire.
- Don't paper over with `--no-verify` or skipping. Fix the root cause.

**Step 2: Run cargo clippy on touched modules**

Run: `cargo clippy --lib -- -D warnings`
Expected: clean.

**Step 3: gamemd parity verification (manual, observation-based)**

This is RE-driven behavior — no automated test fully captures it. Confirm:

- **gamemd reference behavior** (from [SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md](docs/research/SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md)): a harvester driving into a refinery foundation does NOT cause the refinery to move. Buildings are immutable obstacles.
- **Our engine after fix:** same. Confirmed by the integration test in Task 5 plus the safety-net unit test in Task 3.

In-game observation (optional but recommended): launch the engine, build a refinery, send a harvester to dock, verify the refinery sprite stays put through the full dock + unload + undock cycle. Compare to the same operation in gamemd.exe — should be indistinguishable.

**Step 4: No-op commit if anything was tweaked**

If Steps 1-3 surfaced a clippy or test fix, commit it separately. Otherwise no commit needed.

---

## Sources & References

- **Design doc:** [docs/plans/2026-04-28-building-scatter-fix-design.md](docs/plans/2026-04-28-building-scatter-fix-design.md)
- **Ghidra reports:**
  - [SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md](docs/research/SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md) — vtable+0x174 verification, FilterToTechno
  - [CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md](docs/research/CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md) — bit 0x20 vs 0x40
  - [MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md) — radio-driven dock choreography
- **gamemd.exe addresses (kept here, not in Rust comments per memory `feedback_no_engine_refs_in_comments.md`):**
  - `UnitClass::Scatter` = 0x00743A50 (vtable+0x174)
  - `InfantryClass::Scatter` = 0x0051D0D0 (vtable+0x174)
  - `CellClass::Scatter_Objects` = 0x481670
  - `FilterToTechno` rejects RTTI 6 (Building); accepts RTTI 1 (Unit) and 0xF (Infantry)
- **INI keys:** none — this is a runtime occupancy fix, not data-driven
- **Related code:**
  - [src/sim/movement/bump_crush.rs:472-532](src/sim/movement/bump_crush.rs#L472-L532) — `scatter_blocker`
  - [src/sim/movement/movement_occupancy.rs:222-241](src/sim/movement/movement_occupancy.rs#L222-L241) — `OccupiedFriendly` arm
  - [src/sim/pathfinding/cell_entry.rs:148-236](src/sim/pathfinding/cell_entry.rs#L148-L236) — `classify_occupied_cell`, `find_primary_blocker`, `classify_blocker`
  - [src/sim/components.rs:263](src/sim/components.rs#L263) — `MovementTarget.bypass_grid` field
  - [src/sim/miner/miner_dock_sequence.rs:311, :451](src/sim/miner/miner_dock_sequence.rs#L451) — bypass_grid setters
- **Prior commits:**
  - `7375380` — `movement: add bypass_grid flag to MovementTarget`
  - `85fec7b` — `movement: gate path_grid walkability on bypass_grid`
  - `4ea4723` / `17869fe` — `miner: dock-drive direct moves bypass foundation walkability`
  - `f6a20b4` — `miner_tests: add harvester_undocks_through_foundation_to_outside_ore` (the test that didn't catch this bug because it used empty OccupancyGrid)

## Follow-ups (not part of this plan)

- **Approach C from design doc:** track structures in a separate occupancy bit/layer (mirroring gamemd's bit 0x20 vs 0x40 split). Wide refactor across every `OccupancyGrid` consumer. Long-term parity-correct model. File a TODO.
- **`bypass_grid: false` reset during segment repath** at [movement_tick.rs:226](src/sim/movement/movement_tick.rs#L226). Latent issue not relevant to dock case (2-cell paths, no segment repath). File a TODO if other `bypass_grid` use cases ever appear.
