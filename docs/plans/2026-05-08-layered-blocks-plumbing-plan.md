# Layered A* Entity-Block Plumbing Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** At the four `find_move_path` call sites that currently pass `None` (or partial-None) for `ground_blocks` / `bridge_blocks`, pass `Some(&combined_blocks)` for both — mirroring commit 7e35fef item 3 — and add three regression tests pinning the new behavior.

**Architecture:** Pure value-swap at four call sites in `src/sim/movement/`. No signature changes, no new types, no `bump_crush` change. The fix delegates per-layer hard-blocking to `astar_search`'s existing dual-list logic at [src/sim/pathfinding/core.rs:471-483](src/sim/pathfinding/core.rs#L471-L483), which already handles the layered consultation correctly — it just receives `None` for `bridge_blocks` today.

**Design Doc:** [docs/plans/2026-05-08-layered-blocks-plumbing-design.md](docs/plans/2026-05-08-layered-blocks-plumbing-design.md)

---

## Grounding Summary

- **Docs:** `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` §1.1 (cost table), §2.3 (code semantics); `PATHFINDING_ASTAR_GHIDRA_REPORT.md` §6.2, §7.1 (dual closed sets, dual object lists); `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` Phase 5, 6 (bridge cell selection, occupancy re-read).
- **Ghidra (verified live this session):**
  - `UnitClass::Can_Enter_Cell @ 0x73f0a0` LAB_0073f4f9 — `cell+0xE4` (FirstObject ground list) vs `cell+0xE8` (AltObject bridge list) selection confirmed.
  - Bridge-level occupancy re-read at `cell+0x128` triggers when `prevFacing == cell+0x11B + 4` (deck = ground+4). Confirmed.
- **Repo pattern:** mirror commit `7e35fef` item 3 — at [src/sim/movement/movement_path.rs:378-380](src/sim/movement/movement_path.rs#L378-L380), `ground_blocks` was changed `None` → `Some(&combined_blocks)`. We extend the same change to `bridge_blocks` in that site and to both slots at the three remaining sites.
- **INI keys:** none. The fix is pure plumbing — no INI parsing or new constants.
- **Git state:** no commits since the 2026-05-08 design doc touched any of the four files. Premise valid.
- **Still unknown after grounding:** none for this fix. (L9 cross-layer entity_block_map split is a deferred drift, not unknown.)

## Key Technical Decisions

- **Pass `Some(&combined_blocks)` to both `ground_blocks` and `bridge_blocks` slots at all four call sites.** — **Confidence: high**
  - **Source:** Design doc §"Chosen Approach"; mirrors commit 7e35fef item 3.
- **Do not modify `bump_crush::build_entity_block_sets`.** — **Confidence: high**
  - **Source:** Design doc §"Architectural Decisions". `bridge_blocked` BTreeSet stays empty; combined_blocks (= ground_blocked) is fed to both layered slots. In vanilla YR no entity that goes into a hard-block set ever sits on the bridge layer (only structures, which are ground-only).
- **Tests build foundation BTreeSet directly via `crate::sim::production::building_footprint_cells`** rather than constructing a full RuleSet. — **Confidence: high**
  - **Source:** `issue_move_command` accepts `entity_blocks: Option<&BTreeSet<(u16,u16)>>` directly ([movement_commands.rs:60-73](src/sim/movement/movement_commands.rs#L60-L73)). `building_footprint_cells` is the same function `build_entity_block_sets` uses internally for foundation expansion ([bump_crush.rs:142-149](src/sim/movement/bump_crush.rs#L142-L149)).
- **Combine fix + tests into a single commit.** — **Confidence: high**
  - **Source:** matches commit 7e35fef shape (fix + regression tests bundled). Tests pin the fix; landing them separately risks accidental revert.

No low-confidence decisions. `/review-plan` only needs to spot-check the line numbers in Tasks 1-3 and confirm the test scaffolding compiles.

## Open Questions

### Resolved During Planning

- **Q: Does `issue_move_command` accept `entity_blocks` directly, or do I need to construct rules?** → Accepts `entity_blocks: Option<&BTreeSet<(u16,u16)>>` directly. No rules needed for tests.
- **Q: Is the `combined_blocks` local at site 3 the same shape as `merged_entity_blocks_ref` at sites 1/2?** → Yes — both are `&BTreeSet<(u16,u16)>` produced by `merge_path_blocks` from the same `build_entity_block_set(s)` output. Drop-in equivalent.
- **Q: Will the new tests need to tick or just inspect the post-issue path?** → Just inspect for sites 1/2 (initial path is computed inside `issue_move_command`). Site 4 needs ticking (segment exhaustion fires inside `tick_movement_with_grid`).

### Deferred to Implementation

- **Will the new tests reveal any unrelated path-shape regressions?** → Unlikely (the fix is monotonically more conservative — paths only become more avoiding, never less). If a path-shape regression surfaces, it's evidence of an existing latent bug separate from this fix; surface it but don't fix it here.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/movement/movement_path.rs` | Site 3 — bridge_blocks slot in try_repath_after_block |
| Modify | `src/sim/movement/movement_tick.rs` | Site 4 — both slots in segment-exhaustion auto-repath |
| Modify | `src/sim/movement/movement_commands.rs` | Sites 1+2 — both slots in queued append + initial fresh path |
| Modify | `src/sim/movement/movement_tests.rs` | Add 3 regression tests |

## Interface Changes

None. All function signatures preserved. No public API touched.

## Sim Checklist

- [x] All math integer (`BTreeSet<(u16,u16)>` references — no math). No f32/f64.
- [x] No new state added; deterministic state hash unaffected.
- [x] No dependencies on render/ui/sidebar/audio/net.
- [x] Tick ordering unaffected — pathfinder is invoked from movement code, no order change.
- [x] BTreeMap iteration order unaffected — A* operates on PathGrid arrays + the BTreeSet hard-block set, not EntityStore.

## Risk Areas

- **Vehicles on the bridge layer will newly hard-block on cells in `combined_blocks`** (mostly building footprints). Vanilla YR has no buildings under any bridge — no observable regression. Test 4 (optional) pins this side-effect explicitly.
- **Replay determinism:** old replays from before the fix will diverge wherever vehicle paths previously crossed a building (they now route around the first time instead of bumping-and-rerouting). Expected. Acceptable per design doc.
- **Test scaffolding:** the new tests use `issue_move_command` with a manually-built foundation BTreeSet. Confirm the BTreeSet is passed correctly through to `find_move_path` — if it's not (e.g., if `issue_move_command` swallows the arg), the test would pass even with the bug present. Task 4's `cargo check` won't catch this; verify by temporarily reverting Task 1's change and confirming the test FAILS.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Tasks 1-3 | Bridge-layer + ground-layer A* both see structure footprints (code 7) as hard blocks at every find_move_path call site | Without this, vehicles route paths through allied building footprints on initial moves and segment-exhaustion repaths, then bump and re-route — visible every time a player orders a vehicle across their own base | Tests 5-7 below; manual smoke test in Task 12 |
| Tasks 1-3 | combined_blocks fed to bridge_blocks slot — bridge-layer A* gains hard-block awareness | Mirrors gamemd's per-layer object iteration (verified at 0x73f0a0 LAB_0073f4f9). In vanilla, only structures are in combined_blocks, and bridges aren't over buildings, so observable identically to strict layer separation | Test 4 (optional) — synthetic map with bridge over building |
| Test 5 | A path planned across a 2×2 building footprint must NOT visit any of the 4 foundation cells | Player observable: tank ordered across base does not aim through refinery on first plan | Inspect movement_target.path after issue_move_command |
| Test 6 | Queued append path must NOT visit any of the 4 foundation cells | Player observable: shift-clicked queued moves don't aim through buildings | Inspect appended portion of movement_target.path |
| Test 7 | Auto-repath after segment exhaustion must NOT visit any of the 4 foundation cells | Player observable: long-distance vehicle moves don't aim through buildings on the second segment either | Inspect movement_target.path after tick-driven segment exhaustion |

## Sources & References

- **Design doc:** [docs/plans/2026-05-08-layered-blocks-plumbing-design.md](docs/plans/2026-05-08-layered-blocks-plumbing-design.md)
- **Disparity scan:** [docs/gap-scans/2026-05-08-disparity-scan-pathfinding.md](docs/gap-scans/2026-05-08-disparity-scan-pathfinding.md)
- **Ghidra reports cited:**
  - `ra2-rust-game-docs/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` (§1.1, §2.3)
  - `ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md` (§6.2, §7.1)
  - `ra2-rust-game-docs/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` (Phase 5, Phase 6)
- **gamemd.exe addresses (kept here, NOT in Rust comments per project rule):**
  - `UnitClass::Can_Enter_Cell @ 0x73f0a0` — verified live this session. LAB_0073f4f9 selects `cell+0xE4` vs `cell+0xE8` for layer-aware object iteration. Bridge occupancy re-read at `cell+0x128` when `prevFacing == cell+0x11B + 4`.
- **Related code:**
  - [src/sim/pathfinding/core.rs:471-483](src/sim/pathfinding/core.rs#L471-L483) — astar_search per-layer hard-block consultation (the receiver of the fix)
  - [src/sim/movement/bump_crush.rs:112-195](src/sim/movement/bump_crush.rs#L112-L195) — build_entity_block_set(s) (NOT modified)
  - [src/sim/movement/movement_path.rs:27-45](src/sim/movement/movement_path.rs#L27-L45) — merge_path_blocks (unchanged consumer)
- **Prior commits:** `7e35fef` established the fix shape at site 3 (ground_blocks half).

---

## Tasks

### Task 1: Fix site 3 — `try_repath_after_block` bridge_blocks slot

**Why:** Site 3 is the smallest delta — 7e35fef already changed `ground_blocks` from `None` → `Some(&combined_blocks)`; we just extend the same pattern to the adjacent `bridge_blocks` argument. Lowest-risk first to confirm the value-swap pattern works before applying it at the noisier sites.

**Files:**
- Modify: [src/sim/movement/movement_path.rs:380-381](src/sim/movement/movement_path.rs#L380-L381) — change `bridge_blocks` arg from `None` to `Some(&combined_blocks)`

**Pattern:** Direct mirror of commit 7e35fef item 3 (verified by `git show 7e35fef -- src/sim/movement/movement_path.rs` this session).

**Step 1: Apply the edit.**

In [src/sim/movement/movement_path.rs](src/sim/movement/movement_path.rs), find the `find_move_path` call inside `try_repath_after_block` at lines 371-387. The current call passes:

```rust
        terrain_costs,
        Some(&combined_blocks),
        Some(&combined_blocks),
        None,
        zone_mz,
```

Change the third of those (the `bridge_blocks` argument) from `None` to `Some(&combined_blocks)`:

```rust
        terrain_costs,
        Some(&combined_blocks),
        Some(&combined_blocks),
        Some(&combined_blocks),
        zone_mz,
```

The 3-line block of comment above (lines 364-367) explaining the ground_blocks fix can stay — it now applies to bridge_blocks too. Optionally update its wording if you do this task carefully:

```rust
    // The layered A* path consults ground_blocks/bridge_blocks (not entity_blocks)
    // for per-layer hard blocking. Pass the merged set as both ground_blocks and
    // bridge_blocks so the layered search sees structure footprints / stationary
    // obstacles on either layer the same way the flat search does.
```

**Step 2: Verify it compiles.**

Run: `cargo check -p ra2-rust-game --lib`

Expected: PASS. No tests yet — Task 4 runs them.

---

### Task 2: Fix site 4 — segment-exhaustion auto-repath in `movement_tick.rs`

**Why:** Site 4 is the proactive auto-repath fired when a path segment is exhausted (every >24 path steps). Without the fix, the next segment ignores building footprints. Triggers on every long vehicle move.

**Files:**
- Modify: [src/sim/movement/movement_tick.rs:166-189](src/sim/movement/movement_tick.rs#L166-L189) — change `ground_blocks`, `bridge_blocks` args from `None, None` to `Some(mover_entity_blocks_unwrapped), Some(mover_entity_blocks_unwrapped)`

**Pattern:** Mirror of Task 1 + Task 1's documenting comment.

**Step 1: Locate the call.**

The relevant block is inside `auto_repath_path_segment` (or whatever `tick_movement_with_grid` calls — read [movement_tick.rs:166-189](src/sim/movement/movement_tick.rs#L166-L189) to confirm the function name in the current file). The current call passes:

```rust
            if let Some((new_path, new_layers)) = find_move_path(
                ctx,
                layered_pathing_for_seg,
                cur,
                active_layer,
                fg,
                entity_cost_grid,
                mover_entity_blocks,
                None,
                None, // layer-separated entity blocks not yet wired
                seg_zone_mz,
                ...
```

`mover_entity_blocks` is already an `Option<&BTreeSet<(u16,u16)>>`. We want to pass the same value (the combined ground+bridge merged set) for both `ground_blocks` and `bridge_blocks`.

**Step 2: Apply the edit.**

Change the 3 arg lines (`mover_entity_blocks`, `None`, `None`) to:

```rust
                entity_cost_grid,
                mover_entity_blocks,
                mover_entity_blocks,
                mover_entity_blocks,
                seg_zone_mz,
```

And remove the stale comment `// layer-separated entity blocks not yet wired`.

Add a one-line comment explaining the choice (per the design doc — the WHY is non-obvious for a future reader who doesn't see the disparity scan):

```rust
                entity_cost_grid,
                // Pass the merged entity_blocks set to both layered slots so the
                // layered A* sees building footprints regardless of which layer
                // it expands. Mirrors the try_repath_after_block fix.
                mover_entity_blocks,
                mover_entity_blocks,
                mover_entity_blocks,
                seg_zone_mz,
```

**Step 3: Verify it compiles.**

Run: `cargo check -p ra2-rust-game --lib`

Expected: PASS.

---

### Task 3: Fix sites 1+2 — initial move + queued append in `movement_commands.rs`

**Why:** Sites 1 and 2 are the highest-frequency triggers — every initial vehicle move command goes through one of them. Closing them is the main parity restoration.

**Files:**
- Modify: [src/sim/movement/movement_commands.rs:236-256](src/sim/movement/movement_commands.rs#L236-L256) — site 1 (queued append)
- Modify: [src/sim/movement/movement_commands.rs:278-298](src/sim/movement/movement_commands.rs#L278-L298) — site 2 (initial fresh path)

**Pattern:** Same as Tasks 1 and 2.

**Step 1: Edit site 1 (queued append, lines 236-256).**

Current call passes:

```rust
                let Some((appended, appended_layers)) = find_move_path(
                    PathfindingContext {
                        path_grid: Some(grid),
                        zone_grid: None,
                        resolved_terrain,
                    },
                    layered_pathing,
                    append_start,
                    append_layer,
                    effective_target,
                    terrain_costs,
                    merged_entity_blocks_ref,
                    None,
                    None, // Layer-separated blocks not available here
                    zone_mz,
                    ...
```

Change to:

```rust
                let Some((appended, appended_layers)) = find_move_path(
                    PathfindingContext {
                        path_grid: Some(grid),
                        zone_grid: None,
                        resolved_terrain,
                    },
                    layered_pathing,
                    append_start,
                    append_layer,
                    effective_target,
                    terrain_costs,
                    merged_entity_blocks_ref,
                    merged_entity_blocks_ref,
                    merged_entity_blocks_ref,
                    zone_mz,
                    ...
```

Remove the stale comment `// Layer-separated blocks not available here`.

**Step 2: Edit site 2 (initial fresh path, lines 278-298).**

Current call passes the same `merged_entity_blocks_ref, None, None` triple. Apply the identical change:

```rust
    let Some((path, path_layers)) = find_move_path(
        PathfindingContext {
            path_grid: Some(grid),
            zone_grid: None,
            resolved_terrain,
        },
        layered_pathing,
        (start_rx, start_ry),
        current_layer,
        effective_target,
        terrain_costs,
        merged_entity_blocks_ref,
        merged_entity_blocks_ref,
        merged_entity_blocks_ref,
        zone_mz,
        ...
```

Remove the stale comment `// Layer-separated blocks not available here`.

**Step 3: Verify it compiles.**

Run: `cargo check -p ra2-rust-game --lib`

Expected: PASS.

---

### Task 4: Quick regression — pathfinding + movement test suites

**Why:** Confirm the value swap doesn't break any existing test before we add new ones. Catches accidental typos or borrow issues early.

**Step 1: Run pathfinding tests.**

Run: `cargo test -p ra2-rust-game --lib pathfinding`

Expected: ALL PASS. No new tests yet — these are the existing ones from 28f44d0 / 7e35fef / 9ca86d2.

**Step 2: Run movement tests.**

Run: `cargo test -p ra2-rust-game --lib movement`

Expected: ALL PASS.

**Step 3: If any test fails:** STOP. Re-read the failing test. The fix is monotonically more conservative (paths only become more avoiding), so a failure means either:
- A test was relying on the buggy behavior (rare — flag and update assertion to reflect the new, correct behavior).
- Typo in Tasks 1-3 (re-read the diff).

Do NOT proceed to Task 5+ until both suites are green.

---

### Task 5: Add `test_initial_layered_path_avoids_friendly_building_footprint`

**Why:** Pins site 2 (initial fresh path) — the highest-frequency trigger. Without the fix, this test would FAIL because the planned path visits a foundation cell.

**Files:**
- Modify: [src/sim/movement/movement_tests.rs](src/sim/movement/movement_tests.rs) — append to the end of the file (or place near `test_dynamic_occupancy_repath_routes_around_stationary_blocker` for thematic grouping)

**Pattern:** Mirrors `test_dynamic_occupancy_repath_routes_around_stationary_blocker` ([movement_tests.rs:520-576](src/sim/movement/movement_tests.rs#L520-L576)) but inspects the initial path instead of waiting for repath.

**Step 1: Add the test.**

```rust
#[test]
fn test_initial_layered_path_avoids_friendly_building_footprint() {
    // A friendly Drive-locomotor unit ordered across a 2x2 friendly building
    // foundation must plan a path that does NOT visit any foundation cell on
    // the FIRST attempt — gamemd's Can_Enter_Cell returns code 7 (impassable)
    // for unrelated allied buildings, so the layered A* must hard-block them.
    use std::collections::BTreeSet;
    use crate::sim::production::building_footprint_cells;

    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(15, 15);

    // 2x2 friendly building anchored at (5,5) — covers (5,5), (6,5), (5,6), (6,6).
    let foundation: BTreeSet<(u16, u16)> = building_footprint_cells(5, 5, "2x2", &[], &[])
        .into_iter()
        .collect();
    let mut blocks = BTreeSet::new();
    blocks.extend(foundation.iter().copied());

    // Mover at (1,5), goal at (10,5) — straight east through the foundation.
    let mover = GameEntity::test_default(1, "HTNK", "Americans", 1, 5);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (10, 5),
        SimFixed::from_num(1024),
        false,            // queue
        None,             // terrain_costs
        Some(&blocks),    // entity_blocks
        None,             // entity_block_map
        false,            // mover_is_crusher
    ));

    let entity = entities.get(1).expect("mover exists");
    let target = entity
        .movement_target
        .as_ref()
        .expect("initial path was planned");

    for &cell in &target.path {
        assert!(
            !foundation.contains(&cell),
            "Initial path visited foundation cell {:?} — layered A* did not see \
             ground_blocks/bridge_blocks on the first plan. Path: {:?}",
            cell,
            target.path,
        );
    }
    assert_eq!(target.path.first().copied(), Some((1, 5)));
    assert_eq!(target.path.last().copied(), Some((10, 5)));
}
```

**Step 2: Verify.**

Run: `cargo test -p ra2-rust-game --lib test_initial_layered_path_avoids_friendly_building_footprint`

Expected: PASS.

**Step 3 (mandatory sanity check): Confirm the test would FAIL without the fix.**

Temporarily revert Task 3 site 2 (just site 2 — the initial fresh path) by replacing `merged_entity_blocks_ref, merged_entity_blocks_ref, merged_entity_blocks_ref` back to `merged_entity_blocks_ref, None, None`. Re-run the test:

```
cargo test -p ra2-rust-game --lib test_initial_layered_path_avoids_friendly_building_footprint
```

Expected: FAIL. (If it still passes, the test is not actually exercising the fix — likely the `entity_blocks` parameter to `issue_move_command` isn't reaching `find_move_path` as `merged_entity_blocks_ref`. Investigate before proceeding.)

Then RE-APPLY the Task 3 site 2 fix and re-run — expected PASS.

---

### Task 6: Add `test_queued_append_layered_path_avoids_friendly_building_footprint`

**Why:** Pins site 1 (queued append). Triggers when a player shift-clicks queued moves.

**Files:** [src/sim/movement/movement_tests.rs](src/sim/movement/movement_tests.rs)

**Step 1: Add the test.**

```rust
#[test]
fn test_queued_append_layered_path_avoids_friendly_building_footprint() {
    // Issue an initial move, then a queued (queue=true) move that crosses a
    // 2x2 friendly building. The appended portion must avoid the foundation.
    use std::collections::BTreeSet;
    use crate::sim::production::building_footprint_cells;

    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(15, 15);

    let foundation: BTreeSet<(u16, u16)> = building_footprint_cells(5, 5, "2x2", &[], &[])
        .into_iter()
        .collect();
    let mut blocks = BTreeSet::new();
    blocks.extend(foundation.iter().copied());

    // Mover at (1,5). First move to (3,5) (no obstacle). Second move queued
    // to (10,5) — appended portion crosses the foundation.
    let mover = GameEntity::test_default(1, "HTNK", "Americans", 1, 5);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities, &grid, 1, (3, 5),
        SimFixed::from_num(1024),
        false,            // queue=false (initial)
        None, Some(&blocks), None, false,
    ));
    assert!(issue_move_command(
        &mut entities, &grid, 1, (10, 5),
        SimFixed::from_num(1024),
        true,             // queue=true (append)
        None, Some(&blocks), None, false,
    ));

    let entity = entities.get(1).expect("mover exists");
    let target = entity
        .movement_target
        .as_ref()
        .expect("queued path exists");

    for &cell in &target.path {
        assert!(
            !foundation.contains(&cell),
            "Queued append path visited foundation cell {:?}. Path: {:?}",
            cell,
            target.path,
        );
    }
    assert_eq!(target.path.first().copied(), Some((1, 5)));
    assert_eq!(target.path.last().copied(), Some((10, 5)));
}
```

**Step 2: Verify.**

Run: `cargo test -p ra2-rust-game --lib test_queued_append_layered_path_avoids_friendly_building_footprint`

Expected: PASS.

---

### Task 7: Add `test_segment_exhaustion_repath_avoids_friendly_building_footprint`

**Why:** Pins site 4 (auto-repath at segment exhaustion). Triggers on long vehicle moves (>24 path steps). The first segment's plan doesn't see the obstacle (it's beyond 24 steps); the auto-repath at step 24 must see the foundation and route around.

**Files:** [src/sim/movement/movement_tests.rs](src/sim/movement/movement_tests.rs)

**Pattern:** Mirrors `test_segment_exhaustion_triggers_auto_repath` ([movement_tests.rs:922-968](src/sim/movement/movement_tests.rs#L922-L968)).

**Step 1: Add the test.**

```rust
#[test]
fn test_segment_exhaustion_repath_avoids_friendly_building_footprint() {
    // A 38-step path with a 2x2 friendly building at cell 30 (beyond the
    // first 24-step segment). The initial segment doesn't see the foundation;
    // the auto-repath at segment exhaustion must avoid it.
    use std::collections::BTreeSet;
    use crate::sim::production::building_footprint_cells;
    use crate::sim::movement::tick_movement_with_grid;

    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(45, 5);

    // 2x2 friendly building anchored at (30,2) — covers (30,2), (31,2), (30,3), (31,3).
    let foundation: BTreeSet<(u16, u16)> = building_footprint_cells(30, 2, "2x2", &[], &[])
        .into_iter()
        .collect();
    let mut blocks = BTreeSet::new();
    blocks.extend(foundation.iter().copied());

    let mover = GameEntity::test_default(1, "HTNK", "Americans", 1, 2);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (40, 2),
        SimFixed::from_num(15360), // very fast — exhausts segment quickly
        false,
        None,
        Some(&blocks),
        None,
        false,
    ));

    // Tick until the first segment is exhausted and auto-repath fires.
    // Segment is 24 steps; very-fast speed reaches end in ~24 ticks.
    // After segment exhaustion the auto-repath at movement_tick.rs:166
    // computes a fresh path from the unit's current position to (40,2),
    // which crosses the foundation.
    let mut occupancy = OccupancyGrid::rebuild(&entities);
    for _ in 0..40 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut occupancy,
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
    }

    // Inspect the current path (the second segment, post-auto-repath).
    let entity = entities.get(1).expect("mover exists");
    if let Some(target) = entity.movement_target.as_ref() {
        for &cell in &target.path {
            assert!(
                !foundation.contains(&cell),
                "Post-segment-exhaustion repath visited foundation cell {:?}. \
                 Path: {:?}",
                cell,
                target.path,
            );
        }
    }
    // Either the unit reached the goal (route succeeded around the foundation)
    // or it's still en route — both are acceptable. The assertion above is the
    // parity-critical check: the path never visits a foundation cell.
}
```

**Step 2: Verify.**

Run: `cargo test -p ra2-rust-game --lib test_segment_exhaustion_repath_avoids_friendly_building_footprint`

Expected: PASS.

**Step 3 (mandatory sanity check): Confirm the test would FAIL without the fix.**

Temporarily revert Task 2 (movement_tick.rs site 4) back to `mover_entity_blocks, None, None`. Re-run:

```
cargo test -p ra2-rust-game --lib test_segment_exhaustion_repath_avoids_friendly_building_footprint
```

Expected: FAIL. Re-apply the fix and confirm PASS.

---

### Task 8: Run full pathfinding + movement test suites

**Why:** Confirm the new tests don't interact badly with existing tests, and that no existing test regressed.

**Step 1: Run pathfinding tests.**

Run: `cargo test -p ra2-rust-game --lib pathfinding`

Expected: ALL PASS, including the 3 new tests added in Tasks 5-7.

**Step 2: Run movement tests.**

Run: `cargo test -p ra2-rust-game --lib movement`

Expected: ALL PASS.

---

### Task 9: Run full library suite

**Why:** Pathfinding is upstream of many systems. A silent regression in repath behavior could surface as flakes elsewhere. One full sweep to confirm no cross-module impact.

**Step 1: Run.**

Run: `cargo test -p ra2-rust-game --lib`

Expected: ALL PASS.

**Step 2: If any unrelated test fails:** investigate. The fix is contained — unrelated failures suggest either flakiness or a real cross-module dependency we missed. Don't proceed to commit until green.

---

### Task 10: Commit

**Why:** Land fix + tests as a single atomic commit, mirroring the 7e35fef shape.

**Step 1: Stage.**

```
git add src/sim/movement/movement_path.rs \
        src/sim/movement/movement_tick.rs \
        src/sim/movement/movement_commands.rs \
        src/sim/movement/movement_tests.rs
```

**Step 2: Commit.**

```
git commit -m "$(cat <<'EOF'
movement/pathfinding: wire layered hard-blocks at remaining 3 find_move_path call sites

Commit 7e35fef item 3 fixed try_repath_after_block by passing
Some(&combined_blocks) for ground_blocks (bridge_blocks stayed None).
The same shape of bug remained at three other call sites that all
pass None,None for the per-layer block sets:

- movement_commands.rs:236 (queued append)
- movement_commands.rs:278 (initial fresh path)
- movement_tick.rs:166   (segment-exhaustion auto-repath)

Since supports_layered_bridge_pathing returns true for every Drive/Walk/
Mech locomotor, every vehicle initial move and every long-distance
auto-repath went through the layered A* with no hard-block set, routing
through allied building footprints, then bumping and re-routing.

Fix mirrors 7e35fef item 3: pass the merged combined_blocks set to
both ground_blocks and bridge_blocks at all four call sites
(including the bridge slot at try_repath_after_block, which 7e35fef
also left None). In vanilla YR, no entity that goes into the hard-
block set ever sits on the bridge layer, so feeding combined_blocks
to bridge_blocks is observably equivalent to strict per-layer
separation while keeping the diff minimal.

Adds three regression tests pinning each previously-unfixed call site:
- test_initial_layered_path_avoids_friendly_building_footprint
- test_queued_append_layered_path_avoids_friendly_building_footprint
- test_segment_exhaustion_repath_avoids_friendly_building_footprint

Verified at 0x73f0a0 LAB_0073f4f9 that gamemd uses cell+0xE4 (ground)
vs cell+0xE8 (bridge) for layer-aware object iteration; strict per-
layer separation is gamemd's behavior. The cross-layer soft-cost leak
in entity_block_map (un-layered HashMap) is documented as known drift
in the design doc and deferred.

Design doc: docs/plans/2026-05-08-layered-blocks-plumbing-design.md
Plan doc:   docs/plans/2026-05-08-layered-blocks-plumbing-plan.md
EOF
)"
```

**Step 3: Verify the commit.**

Run: `git log --oneline -3`

Expected: HEAD is the new commit; previous two commits unchanged.

---

### Task 11: gamemd.exe in-game parity verification (manual)

**Why:** Unit tests pin the BTreeSet plumbing but the parity bar is "indistinguishable in a single skirmish." This is a manual verification step that requires running both engines.

**Step 1: Pick a comparison scenario.**

Boot a YR skirmish in gamemd.exe and the Rust engine on the same map (e.g. a stock map with mixed terrain — Country Swing or similar). Build a refinery and a war factory in the player's base. Order a Grizzly tank from one side of the base to the other along a path that would naively cross both buildings.

**Step 2: Compare path shapes.**

- gamemd: tank routes around the buildings cleanly on first plan; smooth single-path arc.
- Rust (before this fix, for reference): tank aims through the refinery, bumps, re-routes from inside the cell, may bump the war factory next, repeat.
- Rust (after this fix): same as gamemd — clean first-plan detour.

**Step 3: Document any drift.**

If the Rust path diverges from gamemd's path in a way the player would notice, capture:
- Map name + start/goal cell coordinates
- Screenshot of both paths
- Hypothesis (likely culprit: an unrelated still-deferred item from the disparity scan)

Open a follow-up `/disparity-scan pathfinding` rather than fixing on the spot — the fix is out of this plan's scope.

**Step 4: If no drift observed, mark complete.**

This is the parity bar. Single-skirmish observable similarity is the success condition.

---

## Optional Task 12: Add bridge-layer side-effect test

**Why:** Pins the side-effect of feeding combined_blocks (a ground-layer set) to the bridge_blocks slot. No vanilla map triggers this scenario, but the test locks the expected behavior so a future refactor can't silently regress it.

**Skip this task** unless one of these applies:
- A modder's map is known to put a bridge over a building footprint.
- The next session's brainstorm tightens to Option B (per-layer block split) and you want a baseline test to compare against.

If you skip it, note in the commit message that the behavior is exercised implicitly by Tasks 5-7 — vehicles ordered across a building reach the goal via a route that doesn't visit foundation cells, regardless of which layer they're on at any point.

---

## Post-Plan Self-Review

1. **Spec coverage** — every design-doc requirement has a task: site 3 (Task 1), site 4 (Task 2), sites 1+2 (Task 3), regression suite (Task 4), test 1 (Task 5), test 2 (Task 6), test 3 (Task 7), full suite (Tasks 8-9), commit (Task 10), in-game verification (Task 11). ✓
2. **Placeholder scan** — no TBD/TODO/vague steps. ✓
3. **Architecture check** — all changes mirror commit 7e35fef item 3. No new patterns. ✓
4. **Interface ordering** — no new interfaces. ✓
5. **Risk coverage** — Task 5 step 3 and Task 7 step 3 explicitly verify the test would FAIL without the fix (catches the "test passes but doesn't exercise the fix" failure mode). ✓
6. **Self-containment** — each task names exact files, line ranges, and full code. ✓
7. **Sim/ compliance** — sim checklist in plan header. No render/ui/audio/net deps. ✓
8. **Grounding coverage** — Ghidra (verified live), docs (cited), repo pattern (file/line refs to 7e35fef shape), INI (n/a — confirmed). ✓
9. **Confidence tagging** — all decisions marked high. ✓
10. **Deferred questions** — listed (any unrelated path-shape regressions). ✓
11. **Parity-critical items** — table populated with 5 entries linking tasks to verification. ✓
