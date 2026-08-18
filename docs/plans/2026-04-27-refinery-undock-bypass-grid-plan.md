# Refinery Undock — bypass_grid + A* Start Relaxation — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained, builds independently, and ends with a commit. Do not skip ahead — Task 6 depends on the field defined in Task 1, and Task 8's test depends on Tasks 4-6 all being in place.

**Goal:** Eliminate the harvester head-butt-after-unload bug by replicating gamemd's `BuildingClass::UndockUnit` mechanism — drive the harvester to a fixed point inside the refinery's south-edge foundation cell, then let `Mission_Harvest` State 0 pathfind out to ore from that blocked starting cell.

**Architecture:** Three coordinated changes — a new `bypass_grid` flag on `MovementTarget` that lets dock-sequence direct moves step through blocked foundation cells, a single-line gate in `movement_step.rs` that consults the flag, and removal of the start-cell passability rejection in `astar_search` so the post-undock ore-search A* can find paths from inside the foundation. The flag is wired in BOTH `phase_rotate_to_pad → EnterPad` (for the drive INTO the pad — same blocked-cell issue) and `phase_exit_pad` (for the drive OUT). Plus formula fixes in `refinery_exit_cell` (X AND Y both off by 128 leptons), an `issue_direct_move` rewrite to expand non-unit deltas into Chebyshev unit-step paths (Option B addendum below), updates to pre-existing tests, removal of the misnamed `EXIT_FACING` constant, and DIAG-log cleanup.

## Option B Addendum (added during execution)

After Task 6's Y-formula change exposed a pre-existing bug in `issue_direct_move` (it uses `cell_delta_to_lepton_dir` which only handles unit deltas), the scope was expanded to fix the underlying issue. This is a strict superset of the original Task 6 changes and delivers true endpoint parity across all foundation sizes (not just 4×3 by accidental rounding).

**Three additional changes folded into Task 6:**

1. **X formula fix.** Symmetric to the Y fix — change `(width - 2) * 128` to `(width - 1) * 128`. For 4×3: no observable change (rounds to same cell). For 3×3 / 5×4 / non-default footprints: cell shifts to match gamemd's literal endpoint.

2. **`issue_direct_move` path expansion.** New helper `expand_path_unit_steps(start, end)` that produces a Chebyshev-stepped path where each consecutive pair has unit delta. Replaces `path: vec![start, target]` with the expanded path so `cell_delta_to_lepton_dir` (which only supports unit deltas) works correctly for every step. Each intermediate cell is foundation-interior; `bypass_grid=true` handles walkability.

3. **Test update.** `refinery_pad_and_exit_cells` 3×3 case asserts `(6, 7)` instead of `(5, 7)` — the X-formula fix shifts the 3×3 exit cell.

**Stepping policy:** Chebyshev (each step takes unit delta in `(sign(dx), sign(dy))` until reaching destination). Diagonal moves are natural; cardinal moves are unit-delta. Reuses the proven cell-crossing logic; no new movement-system code paths.

**Behavior of the 3 callers of `issue_direct_move` after the change:**
- `phase_rotate_to_pad` (queue → pad): always unit delta, expansion is no-op.
- `phase_exit_pad` (pad → exit): multi-cell delta now expands properly. With `bypass_grid=true`, all intermediate cells are walkable for this entity.
- `handle_move_to_ore` ore-adjacent step: gated by `dx<=1 && dy<=1`, always unit delta. Expansion is no-op.

**Design Doc:** [docs/plans/2026-04-27-refinery-undock-bypass-grid-design.md](2026-04-27-refinery-undock-bypass-grid-design.md)

---

## Grounding Summary

The design doc already establishes the grounding from a Ghidra audit performed during the brainstorm:

- **Docs cited:** `HARVESTER_DOCK_UNLOAD.md §4` (audited — found errors documented in design), `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md §6` (audited — disagrees with §4 on coord-getter, sibling is wrong), `PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.1` (verified A* has no start-cell passability check), `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md §State 0` (verified post-undock flow).
- **Ghidra verification this session:** `BuildingClass::UndockUnit @ 0x004593A0`, `BuildingClass::GetCoords @ 0x00447ac0` (vtable+0x48), `DriveLocomotionClass::Force_Track @ 0x004B0C40` (vtable+0x70). Verified Force_Track sets `head_to`/`destination` to `coord+(-128,+128)` and writes the first arg to locomotor field +0x54 (purpose unverified — confirmed NOT a screen-facing).
- **Repo pattern mirrored:** `bypass_grid` field follows the same shape as the existing `MovementTarget::ignore_terrain_cost` field (bool, default false, read at movement-tick time, gated by `#[serde(default)]` for snapshot compat).
- **INI keys:** none new. The fix uses existing INI-driven foundation dimensions (parsed via `art(md).ini`).
- **Still unknown after grounding:** purpose of locomotor field +0x54 (the `0x47` value gamemd writes there). Per design, ignored — it's not a facing and no caller reads it as one.

## Key Technical Decisions

- **Add `bypass_grid` as a parallel field to `ignore_terrain_cost` rather than extending the existing flag's semantics.** — Keeps the two concerns (terrain-cost vs path-grid blocking) separable. The existing `ignore_terrain_cost` user (`handle_move_to_ore` post-A* ore approach) needs only the terrain bypass. **Confidence:** high. **Source:** design doc Alternatives section (alternative C rejected for hidden coupling).

- **Remove A* start-passable rejection unconditionally rather than gating it on a flag.** — gamemd's A* has no such rejection; ours added it for symmetry with an unrelated "destination must be passable" check, but the symmetry is wrong. The relaxation is a strict correctness improvement; no caller depends on the old behavior. **Confidence:** high. **Source:** PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.1, mental scan of A* callers in design doc.

- **Exit cell `(rx+1, ry+2)` for 4×3 GAREFN matches gamemd's literal endpoint.** — Verified via Ghidra decompilation of `BuildingClass::GetCoords` and the offset arithmetic in `BuildingClass::UndockUnit`. **Confidence:** high. **Source:** Ghidra audit; design doc Architecture Context.

- **Drop `EXIT_FACING = 0x47` constant entirely rather than re-purposing it.** — Per audit, `0x47` is not a screen-facing in gamemd. Writing it as `facing_target` produces wrong visual (harvester points ESE while moving SW). Letting the locomotor compute facing from movement direction is correct. **Confidence:** high. **Source:** Ghidra audit (Force_Track decompilation).

## Open Questions

### Resolved During Planning

- *Should `bypass_grid` also bypass entity occupancy?* — No. Entity occupancy is checked in separate code paths in `movement_step` that don't read this flag. Mirrors gamemd's `is_on_track` which still respects unit collisions. Resolved in design doc Risk Areas.
- *Should the alt-layer fallback in `astar_search` (lines 294-304) be preserved?* — No. The fallback only existed to retry on the alternate layer when the start cell was blocked on the primary layer. With the start-rejection removed, A* expands neighbors normally on the primary layer; bridge-vs-ground layer transitions happen organically during expansion. Resolved in design doc Components section.

### Deferred to Implementation

- *Exact verification cell sequence in the new `test_harvester_undocks_through_foundation_to_outside_ore` test.* — Will depend on the deterministic A* expansion order from `(rx+1, ry+2)`. The assertions name structural facts (state transitions, cells reached) rather than exact intermediate cells.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/components.rs:194-257` | Add `bypass_grid: bool` field to `MovementTarget` |
| Modify | `src/sim/movement/movement_step.rs:446` | Gate `path_grid.is_walkable` on `target.bypass_grid` |
| Modify | `src/sim/movement/movement_tick.rs:221` | Defensive reset `bypass_grid: false` in segment-replan rebuild |
| Modify | `src/sim/pathfinding/core.rs:262-305` | Remove start-passable rejection in `astar_search` |
| Modify | `src/sim/pathfinding/core_tests.rs:160-166` | Flip `test_find_path_blocked_start` + add "all neighbors blocked" test |
| Modify | `src/sim/miner/miner_dock_sequence.rs` (lines 42 const, 98-111 fn, 298-315 fn, 422-471 fn) | Fix exit-cell Y formula, mirror bypass_grid in phase_rotate_to_pad AND phase_exit_pad, drop EXIT_FACING |
| Modify | `src/sim/miner/miner_tests.rs` (lines 1077, 1175, 1210, 1264) | Update 4 tests that hardcode old `(11, 11)` exit cell to new `(11, 12)` |
| Modify | `src/sim/miner/miner_system.rs` | Strip all `DIAG[...]` log lines (15 sites: 193, 263, 288, 311, 332, 348, 359, 376, 392, 412, 422, 449, 460, 475, 492) |
| Modify | `src/sim/miner/miner_dock_sequence.rs:463-469` | Strip `DIAG[exit_pad_arrival]` log line |
| Modify | `src/sim/miner/miner_tests.rs` | Add `harvester_undocks_through_foundation_to_outside_ore` |

## Interface Changes

- `MovementTarget` gains a public field `bypass_grid: bool`. Existing `Default` impl, `Clone`, `Serialize`, `Deserialize` derives all continue to work; `#[serde(default)]` on the new field keeps old snapshots loadable.
- `astar_search` (and `find_path`, `find_path_with_costs`, etc. that wrap it) become more permissive: they now accept blocked start cells and return `Some(path)` if any neighbor is walkable, where they previously returned `None`. No call signature changes; behavior change documented in the function doc-comment update.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 introduced (the new field is a bool; A* changes are integer index ops).
- [x] New state included in deterministic state hash — `bypass_grid` is part of `MovementTarget` which is already serialized as part of `GameEntity`. No additional hash plumbing needed; the bool is naturally captured.
- [x] No dependencies on render/ui/sidebar/audio/net — all changes are in `sim/components.rs`, `sim/movement/`, `sim/pathfinding/`, `sim/miner/`.
- [x] Tick ordering impact: none. Changes operate inside existing `tick_miners` and `tick_movement` slots.
- [x] BTreeMap iteration order: not affected. No new entity-iteration code.

## Risk Areas

From design Impact Analysis:

1. **A* start relaxation has the broadest blast radius.** Every A* caller is affected. Mitigation: the change is a strict correctness improvement (closer to gamemd, never returns a worse path). New negative test (`test_find_path_blocked_start_all_neighbors_blocked_returns_none`) pins the boundary case.
2. **`bypass_grid` MUST NOT bypass occupancy.** Verified at design time; must be re-verified during Task 2 by reading the surrounding code in `movement_step.rs` to confirm occupancy checks are in separate code paths.
3. **Snapshot serialization compat.** `#[serde(default)]` on the new field makes old snapshots load with `bypass_grid = false`, matching pre-fix behavior. Verify by checking `MovementTarget`'s existing serde annotations.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | A* allows blocked-start | gamemd's `AStar_main_loop` has no start-cell passability check. Without this, our harvester can't pathfind out of the foundation post-undock — same head-butt symptom in a new location. | PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.1; new test `test_find_path_blocked_start_finds_path_through_walkable_neighbors` |
| Task 6 | Exit cell Y formula matches gamemd | gamemd's `BuildingClass::GetCoords + (-128, +128)` lands at cell `(rx+1, ry+2)` for 4×3, not `(rx+1, ry+1)`. The off-by-one was introducing an undock-position drift visible in side-by-side. | Ghidra decompilation of `GetCoords @ 0x447ac0`; arithmetic worked out in design doc |
| Task 6 | `EXIT_FACING = 0x47` removed | Per Ghidra audit, `0x47` is NOT a screen-facing — it's a value gamemd stores at locomotor field +0x54 of unverified purpose. Setting it as our `facing_target` produces a wrong visual (harvester points ESE while moving SW). | Force_Track decompilation in audit; new behavior leaves facing to natural source-to-dest derivation |
| Task 6 | `bypass_grid = true` in phase_exit_pad | gamemd's `is_on_track` mode skips Can_Enter_Cell during the brief drive. Without our mirror, harvester head-butts the foundation. | Force_Track decompilation; new test `harvester_undocks_through_foundation_to_outside_ore` |
| Task 6 | `bypass_grid = true` in phase_rotate_to_pad → EnterPad | The dock ENTRY drive has the same shape as the exit drive — `issue_direct_move(pad)` to a foundation-blocked cell. Same bug class on the entry side; same fix. Existing tests don't catch it because `tick_miners_n` uses `tick_movement` (no path_grid). | Same Force_Track audit; covered by the same end-to-end test (which exercises full drive-into-foundation behavior). |

---

## Tasks

### Task 1: Add `bypass_grid` field to `MovementTarget`

**Why:** Foundational — defines the new flag that Tasks 2, 3, and 6 consume. Must come first.

**Files:**
- Modify: `src/sim/components.rs:194-257` (the `MovementTarget` struct + `Default` impl)

**Pattern:** Mirrors the existing `ignore_terrain_cost` field already on `MovementTarget` (declaration shape, default value, doc-comment style, position relative to other fields).

**Step 1: Add the field declaration**

Inside the `MovementTarget` struct (between `ignore_terrain_cost` and the closing brace, around line 256):

```rust
    /// When true, the movement tick skips PathGrid walkability checks for cell entry.
    /// Used by dock-sequence direct moves where the harvester must traverse the
    /// refinery foundation footprint (cells marked blocked by block_building_footprint).
    /// Does NOT bypass entity occupancy checks — other movers still collide.
    #[serde(default)]
    pub bypass_grid: bool,
```

**Step 2: Add the field to the `Default` impl**

In the `Default for MovementTarget` impl block (around line 261-282), add to the struct literal:

```rust
            bypass_grid: false,
```

Match the alphabetical/positional ordering of other fields — place adjacent to `ignore_terrain_cost: false`.

**Step 3: Verify**

Run: `cargo check --workspace`
Expected: PASS — adding a field with `#[serde(default)]` and a default value should compile cleanly. No call sites broken.

Run: `cargo build --workspace`
Expected: PASS.

**Step 4: Commit**

```
git add src/sim/components.rs
git commit -m "movement: add bypass_grid flag to MovementTarget

Lets dock-sequence direct moves step through PathGrid-blocked cells
(refinery foundation cells) without bypassing entity occupancy.
Default false; existing callers unaffected."
```

---

### Task 2: Gate `path_grid.is_walkable` on `bypass_grid` in movement_step

**Why:** This is the single-line behavior change that makes the new flag DO something. Order: after Task 1 (field must exist).

**Files:**
- Modify: `src/sim/movement/movement_step.rs:446`

**Pattern:** Mirrors the existing short-circuit pattern used for `terrain_ok` immediately below at line 448-450 (where `target.ignore_terrain_cost` short-circuits the cost-grid check).

**Step 1: Read the surrounding code to confirm scope**

Open `src/sim/movement/movement_step.rs:420-475`. Confirm:
- Line 446 is inside the `MovementLayer::Ground` arm of the `match next_layer { ... }` block (around lines 428-447).
- The line currently reads (modulo whitespace):
  ```rust
                  path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))
  ```
- The expression sits inside an `else` branch of an `if is_water_mover { ... } else { ... }` — only the non-water-mover path is affected by this change.

**Critical re-verify:** confirm by reading lines 470-525 that the entity-occupancy / `mover_entity_blocks` / `mover_entity_block_map` checks are in separate code paths from the path_grid check. If `bypass_grid` accidentally bypasses occupancy too, the design intent is broken.

**Step 2: Apply the gate**

Change line 446 from:

```rust
                    path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))
```

to:

```rust
                    target.bypass_grid || path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))
```

**Step 3: Verify**

Run: `cargo check --workspace`
Expected: PASS.

Run: `cargo test movement -- --skip exit_pad`
Expected: PASS — existing movement tests don't set `bypass_grid`, so behavior unchanged for them.

**Step 4: Commit**

```
git add src/sim/movement/movement_step.rs
git commit -m "movement: gate path_grid walkability on bypass_grid

Lets a MovementTarget with bypass_grid=true step through cells the
PathGrid marks unwalkable. Entity occupancy checks (separate code
paths) still apply — the bypass is scoped to terrain/footprint
blocking only."
```

---

### Task 3: Defensive reset of `bypass_grid` in segment-replan rebuild

**Why:** When `movement_tick` rebuilds a `MovementTarget` after exhausting a 24-step path segment, the rebuilt target should have `bypass_grid: false` (matching the existing reset pattern for `ignore_terrain_cost`). Direct moves don't trigger segment replan in practice (their 2-cell paths are always shorter than 24 steps), but defensive resets prevent the flag from sticking around if a future change makes that possible. Order: after Task 1 (field must exist).

**Files:**
- Modify: `src/sim/movement/movement_tick.rs:221`

**Pattern:** Mirrors the existing `ignore_terrain_cost: false` reset on the same line.

**Step 1: Locate the reset block**

Open `src/sim/movement/movement_tick.rs` around line 215-225. Confirm there's a `MovementTarget { ... }` struct-literal rebuild containing the line `ignore_terrain_cost: false,` at line 221.

**Step 2: Add the reset**

Adjacent to `ignore_terrain_cost: false,`, add:

```rust
                        bypass_grid: false,
```

Position it next to `ignore_terrain_cost` to keep the two related flags grouped.

**Step 3: Verify**

Run: `cargo check --workspace`
Expected: PASS.

**Step 4: Commit**

```
git add src/sim/movement/movement_tick.rs
git commit -m "movement: reset bypass_grid in segment-replan rebuild

Defensive — direct_move's 2-cell paths don't trigger segment replan
in practice, but the rebuilt MovementTarget should match the same
reset pattern as ignore_terrain_cost."
```

---

### Task 4: Remove A* start-passable rejection in `astar_search`

**Why:** Without this change, the post-undock `Mission_Harvest` ore search can't find a path FROM inside the foundation TO ore — A* returns None and the harvester sits stuck (same symptom as the head-butt, just at a different location). Order: after Tasks 1-3 (independent, but the integration test in Task 8 depends on this AND Task 6).

**Files:**
- Modify: `src/sim/pathfinding/core.rs:292-305`

**Pattern:** Removes a Rust-only addition that has no gamemd equivalent. The function's doc-comment should call out the matching-gamemd intent.

**Step 1: Locate the rejection block**

Open `src/sim/pathfinding/core.rs` around line 262-305. Find the block that currently reads:

```rust
    if !start_passable {
        // Fallback: try the other layer (matches old find_layered_path fallback)
        let alt_layer = if start_layer == MovementLayer::Bridge {
            MovementLayer::Ground
        } else {
            MovementLayer::Bridge
        };
        let alt_passable = grid.is_walkable_on_layer(start.0, start.1, alt_layer);
        if !alt_passable {
            return None;
        }
        // Recurse with flipped layer
        return astar_search(grid, start, alt_layer, goal, options);
    }
```

**Step 2: Replace with a comment block explaining the new behavior**

Replace the entire `if !start_passable { ... }` block (lines 292-305) with:

```rust
    // Start cell may be blocked (e.g. unit standing inside a building footprint
    // after undock). Matches the original engine's A*: the start node is seeded
    // into the open set without a passability check; only neighbor expansion
    // calls Can_Enter_Cell. If all 8 neighbors are also blocked, the open set
    // exhausts naturally and we return None below.
    let _ = start_passable; // suppress unused-variable warning
```

If `start_passable` is the ONLY use of the local computed at lines 272-291, remove that computation block entirely and drop the `let _ = start_passable;` line. Verify by reading lines 272-291: the variable is computed via a multi-branch conditional that calls `is_at_bridge_level`, `is_cell_passable_for_mover`, and `is_walkable_on_layer`. If `start_passable` is ONLY consumed by the rejection block, delete lines 272-291 entirely.

**Critical:** confirm by re-reading lines 272-291 that no other code below uses `start_passable`. The change should leave the file shorter, not just functionally equivalent.

**Step 3: Update the function doc-comment**

The current doc-comment at lines 253-261 says:

```rust
/// Unified A* search with height-based bridge routing.
///
/// Matches gamemd.exe's single AStar_main_loop (0x00429a90). Uses dual closed
/// lists (ground/bridge) per cell, with closed-list selection based on the
/// CURRENT node's height vs neighbor's ground_level (not computed neighbor height).
///
/// Always returns `Vec<LayeredPathStep>` with per-cell layer info derived from
/// height comparison. Thin public wrappers extract `(u16, u16)` for callers that
/// don't need layer info.
```

Append at the end of the doc-comment:

```rust
///
/// Accepts blocked start cells: a unit standing in an impassable cell (e.g.
/// inside a building footprint) can still pathfind out via any walkable
/// neighbor. Returns `None` only when the goal is unreachable from any
/// neighbor of the start.
```

Per the CLAUDE.md memory `feedback_no_engine_refs_in_comments`, do NOT cite the gamemd address (0x429a90) in the new comment — but the existing line that already cites it can stay (it's a pre-existing reference, not a new addition). Just don't ADD new gamemd address references in this change.

**Step 4: Verify build**

Run: `cargo check --workspace`
Expected: PASS.

Run: `cargo clippy --all-targets`
Expected: PASS — no new warnings (no unused variables if the cleanup in Step 2 was done correctly).

**Step 5: Verify tests**

Run: `cargo test pathfinding -- --skip blocked_start`
Expected: PASS — all non-blocked-start A* tests should still pass. Blocked-start test will fail in Task 5; that's expected.

**Step 6: Commit**

```
git add src/sim/pathfinding/core.rs
git commit -m "pathfinding: allow A* to start from a blocked cell

Removes a Rust-only rejection check. The original engine's A* seeds
the start node into the open set without a passability check; only
neighbor expansion checks Can_Enter_Cell. This lets a harvester
sitting inside a building footprint pathfind out via any walkable
neighbor — the post-undock case.

test_find_path_blocked_start (which asserted the old rejection
behavior) will be flipped in the next commit."
```

---

### Task 5: Flip `test_find_path_blocked_start` and add a negative test

**Why:** The existing test asserts the old rejection behavior. With Task 4 in place, the test fails (the function now returns Some, not None). Flipping the assertion + adding a negative test pins the new contract: blocked-start succeeds when neighbors are walkable, fails when all neighbors are also blocked. Order: after Task 4 (depends on the new A* behavior).

**Files:**
- Modify: `src/sim/pathfinding/core_tests.rs:160-166`

**Pattern:** Standard A* test pattern in this file — construct a small `PathGrid`, set blocked cells, call `find_path`, assert on the result.

**Step 1: Replace the existing test**

Find the existing test at line 160-166:

```rust
#[test]
fn test_find_path_blocked_start() {
    let mut grid: PathGrid = PathGrid::new(10, 10);
    grid.set_blocked(0, 0, true);
    let path: Option<Vec<(u16, u16)>> = find_path(&grid, (0, 0), (5, 5));
    assert!(path.is_none(), "Blocked start should return None");
}
```

Replace with:

```rust
#[test]
fn test_find_path_blocked_start_finds_path_through_walkable_neighbors() {
    // Matches gamemd's A*: the start cell may be blocked (e.g. a unit
    // standing inside a building footprint after undock). A* expands
    // neighbors normally; if any neighbor is walkable, a path can be
    // found.
    let mut grid: PathGrid = PathGrid::new(10, 10);
    grid.set_blocked(5, 5, true);
    let path: Option<Vec<(u16, u16)>> = find_path(&grid, (5, 5), (8, 5));
    assert!(
        path.is_some(),
        "Blocked start with walkable neighbors should find a path"
    );
    let path = path.unwrap();
    assert_eq!(path.first().copied(), Some((5, 5)), "path starts at start cell");
    assert_eq!(path.last().copied(), Some((8, 5)), "path ends at goal");
    assert!(path.len() >= 2, "path has at least start + goal");
}

#[test]
fn test_find_path_blocked_start_all_neighbors_blocked_returns_none() {
    // Negative case: if the start cell is blocked AND all 8 neighbors
    // are blocked, A* exhausts its open set and returns None.
    let mut grid: PathGrid = PathGrid::new(10, 10);
    grid.set_blocked(5, 5, true);
    grid.set_blocked(4, 4, true);
    grid.set_blocked(5, 4, true);
    grid.set_blocked(6, 4, true);
    grid.set_blocked(4, 5, true);
    grid.set_blocked(6, 5, true);
    grid.set_blocked(4, 6, true);
    grid.set_blocked(5, 6, true);
    grid.set_blocked(6, 6, true);
    let path: Option<Vec<(u16, u16)>> = find_path(&grid, (5, 5), (8, 5));
    assert!(
        path.is_none(),
        "Blocked start with all neighbors blocked should return None"
    );
}
```

**Step 2: Verify**

Run: `cargo test pathfinding::core_tests::test_find_path_blocked_start`
Expected: PASS — both the renamed test and the new negative test should pass.

Run: `cargo test pathfinding`
Expected: ALL PASS.

**Step 3: Commit**

```
git add src/sim/pathfinding/core_tests.rs
git commit -m "pathfinding: pin new blocked-start A* contract via tests

Flips test_find_path_blocked_start to assert the new positive case
(blocked start with walkable neighbors → path found) and adds a
negative test for the boundary (all neighbors blocked → None)."
```

---

### Task 6: Fix `refinery_exit_cell` Y formula, wire `bypass_grid` in dock-drive phases, drop EXIT_FACING, update tests

**Why:** This is the actual fix for the user-visible bug. Brings together: (a) correct exit-cell formula matching gamemd's literal endpoint, (b) wiring the new `bypass_grid` flag in the two `direct_move`-into-foundation sites (`phase_rotate_to_pad → EnterPad` AND `phase_exit_pad`), (c) removing the `EXIT_FACING = 0x47` constant (per audit, not a facing), (d) updating four pre-existing tests that hardcode the old `(11, 11)` exit cell. Order: after Tasks 1, 2, 4 (depends on the field, the gate, AND the relaxed A*).

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs` — four sub-changes in the same file:
  1. `EXIT_FACING` constant at line 42 (with doc comment lines 39-41)
  2. `refinery_exit_cell` function (lines 98-111)
  3. `phase_rotate_to_pad` function (lines 298-315) — mirror bypass_grid wiring
  4. `phase_exit_pad` function (lines 422-471)
- Modify: `src/sim/miner/miner_tests.rs` — four pre-existing tests that hardcode the old exit cell:
  1. `refinery_pad_and_exit_cells` (line 1077)
  2. `exit_pad_clears_ore_targets_on_arrival` (line 1175)
  3. `exit_pad_blocks_transition_during_teleport` (line 1210)
  4. `chrono_miner_archive_cleared_after_undock_picks_new_target` (line 1264)

**Pattern:** Existing `refinery_exit_cell` formula style (integer arithmetic over leptons, clamped via `.max(0) as u16`); existing `phase_exit_pad` write-back-to-entity pattern using `sim.entities.get_mut(...).movement_target`.

**Step 1: Locate and remove the `EXIT_FACING` constant**

Find the line that declares `EXIT_FACING`:

```rust
const EXIT_FACING: u8 = 0x47;
```

Delete that line. Also delete its doc comment (typically 1-3 lines above) if any.

**Step 2: Fix `refinery_exit_cell` Y formula**

Find the function `refinery_exit_cell` at lines 98-111. The current body is:

```rust
pub(super) fn refinery_exit_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    _queueing_cell: Option<(u16, u16)>,
) -> (u16, u16) {
    // Building center in leptons: (rx*256 + (w-1)*128, ry*256 + (h-1)*128).
    // UndockUnit offset: (-128, +128) leptons from center.
    // Combined and divided by 256 for cell coordinates.
    let exit_x = (rx as i32 * 256 + (width as i32 - 2) * 128) / 256;
    let exit_y = (ry as i32 * 256 + height as i32 * 128) / 256;
    (exit_x.max(0) as u16, exit_y.max(0) as u16)
}
```

Replace the body with:

```rust
pub(super) fn refinery_exit_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    _queueing_cell: Option<(u16, u16)>,
) -> (u16, u16) {
    // Building center in leptons (BuildingClass::GetCoords convention):
    //   X = origin.X + (w-1)*128 = rx*256 + 128 + (w-1)*128
    //   Y = origin.Y + (h-1)*128 = ry*256 + 128 + (h-1)*128
    // UndockUnit offset (-128, +128) leptons from center, then floor-divide
    // by 256 for cell coordinates. Lands at the south-edge interior cell of
    // the foundation (e.g. (rx+1, ry+2) for 4x3).
    let exit_x = (rx as i32 * 256 + (width as i32 - 2) * 128) / 256;
    let exit_y = (ry as i32 * 256 + height as i32 * 128 + 128) / 256;
    (exit_x.max(0) as u16, exit_y.max(0) as u16)
}
```

Three changes in the body: comment block updated to reflect the verified formula, and the `exit_y` numerator gains `+ 128` (the UndockUnit offset). `exit_x` formula is unchanged.

Per CLAUDE.md memory `feedback_no_engine_refs_in_comments`, the new comment doesn't cite the gamemd function name `BuildingClass::GetCoords` — only describes the formula and the resulting cell. Adjust the comment to:

```rust
    // Building center in leptons (foundation geometric center):
    //   X = rx*256 + 128 + (w-1)*128
    //   Y = ry*256 + 128 + (h-1)*128
    // Offset (-128, +128) leptons from center, then floor-divide by 256 for
    // cell coordinates. Lands at the south-edge interior cell of the
    // foundation (e.g. (rx+1, ry+2) for 4x3).
```

**Step 3: Update `phase_exit_pad` — set bypass_grid, drop facing_target write**

Find `phase_exit_pad` at lines 422-471. Locate the block that currently reads (around lines 441-448):

```rust
    if !moving && !at_exit {
        // Issue the exit move and set facing to match original engine.
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.facing_target = Some(EXIT_FACING);
        }
        return;
    }
```

Replace with:

```rust
    if !moving && !at_exit {
        // Issue the exit move with bypass_grid so the harvester can step
        // through foundation cells (marked unwalkable in path_grid). Facing
        // is left to the locomotor's natural source-to-dest derivation.
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            if let Some(ref mut mt) = entity.movement_target {
                mt.bypass_grid = true;
            }
        }
        return;
    }
```

**Step 3b: Mirror `bypass_grid` wiring in `phase_rotate_to_pad`**

`phase_rotate_to_pad` at lines 298-315 issues the SAME pattern: `issue_direct_move(pad)` where `pad` is inside the foundation footprint. Without `bypass_grid`, the harvester would head-butt during dock ENTRY just as it does during dock EXIT (the bug is symmetric; we just haven't observed the entry-side variant because tests don't exercise `tick_movement_with_grid`). Apply the parallel fix.

Locate the block at lines 307-315 in `phase_rotate_to_pad`:

```rust
    if apply_rotation(sim, snap.entity_id, target_facing, rot) {
        // Rotation complete — issue a direct move onto the pad cell.
        // The pad is inside the building footprint so A* can't reach it;
        // issue_direct_move bypasses pathfinding (matches original engine's
        // ILocomotion::MoveTo with speed 1.0).
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, pad, snap.speed);
        snap.miner.dock_phase = RefineryDockPhase::EnterPad;
    }
```

Replace with:

```rust
    if apply_rotation(sim, snap.entity_id, target_facing, rot) {
        // Rotation complete — issue a direct move onto the pad cell with
        // bypass_grid so the harvester can step into the foundation footprint.
        // (issue_direct_move alone only bypasses A*; the per-tick walkability
        // check in movement_step would otherwise reject entry into the
        // PathGrid-blocked pad cell.)
        movement::issue_direct_move(&mut sim.entities, snap.entity_id, pad, snap.speed);
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            if let Some(ref mut mt) = entity.movement_target {
                mt.bypass_grid = true;
            }
        }
        snap.miner.dock_phase = RefineryDockPhase::EnterPad;
    }
```

**Step 4: Verify file is clean**

Search the file for any remaining references to `EXIT_FACING`:

Run: `cargo check --workspace`

If there's a stale reference to `EXIT_FACING`, the compiler will flag it. Remove any reference found.

Run: `cargo build --workspace`
Expected: PASS.

**Step 4.5: Update four pre-existing tests that hardcode the old exit cell**

The Y-formula change in Step 2 moves the exit cell from `(rx+1, ry+1)` to `(rx+1, ry+2)` for a 4×3 GAREFN at `(rx, ry)`. Four tests in `src/sim/miner/miner_tests.rs` hardcode the old value and will fail without these mechanical updates:

**(a) `refinery_pad_and_exit_cells` (line 1077-1097):**

- Line 1082 comment — change `(11, 11)` → `(11, 12)`:
  ```rust
  // Before:
  // exit = building_center + (-0x80, +0x80) leptons = (11, 11)
  // After:
  // exit = building_center + (-0x80, +0x80) leptons = (11, 12)
  ```
- Line 1085:
  ```rust
  // Before:
  assert_eq!(refinery_exit_cell(10, 10, 4, 3, None), (11, 11));
  // After:
  assert_eq!(refinery_exit_cell(10, 10, 4, 3, None), (11, 12));
  ```
- Line 1089 comment — change `(5, 6)` → `(5, 7)`:
  ```rust
  // Before:
  // exit = building_center + (-0x80, +0x80) leptons = (5, 6)
  // After:
  // exit = building_center + (-0x80, +0x80) leptons = (5, 7)
  ```
- Line 1092:
  ```rust
  // Before:
  assert_eq!(refinery_exit_cell(5, 5, 3, 3, None), (5, 6));
  // After:
  assert_eq!(refinery_exit_cell(5, 5, 3, 3, None), (5, 7));
  ```

**(b) `exit_pad_clears_ore_targets_on_arrival` (line 1175):**

- Line 1181 comment:
  ```rust
  // Before:
  // Refinery at (10, 10). Exit cell for a 4x3 foundation = (11, 11).
  // After:
  // Refinery at (10, 10). Exit cell for a 4x3 foundation = (11, 12).
  ```
- Line 1183:
  ```rust
  // Before:
  let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 11);
  // After:
  let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 12);
  ```

**(c) `exit_pad_blocks_transition_during_teleport` (line 1210):**

- Line 1219:
  ```rust
  // Before:
  let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 11);
  // After:
  let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 12);
  ```

**(d) `chrono_miner_archive_cleared_after_undock_picks_new_target` (line 1264):**

- Line 1270 comment:
  ```rust
  // Before:
  // Refinery at (10, 10), 4x3 foundation. Exit cell = (11, 11).
  // After:
  // Refinery at (10, 10), 4x3 foundation. Exit cell = (11, 12).
  ```
- Lines 1273-1275 comment — `distance 4 from exit` → `distance ~4 from exit` (sqrt(16+1) ≈ 4.12, still inside `local_continuation_radius`=6):
  ```rust
  // Before:
  // Place ONE ore patch at (15, 11): distance 4 from exit, within
  // local_continuation_radius (default 6). This is what the fresh local
  // scan from current position should pick.
  // After:
  // Place ONE ore patch at (15, 11): distance ~4 from exit (sqrt(16+1)),
  // within local_continuation_radius (default 6). This is what the fresh
  // local scan from current position should pick.
  ```
- Line 1284 comment:
  ```rust
  // Before:
  // Spawn miner at exit cell (11, 11), mid-ExitPad. ...
  // After:
  // Spawn miner at exit cell (11, 12), mid-ExitPad. ...
  ```
- Line 1291:
  ```rust
  // Before:
  let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 11);
  // After:
  let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 12);
  ```

These updates are part of the SAME logical change as Task 6 — the test expectations track the formula. Stage them in the same commit.

**Step 5: Run miner tests**

Run: `cargo test miner`

Expected: PASS for all updated tests. Specifically:
- `refinery_pad_and_exit_cells` — updated assertions match new formula.
- `exit_pad_clears_ore_targets_on_arrival` — miner now spawned at the new exit cell `(11, 12)`, arrival branch fires, cleanup runs, assertions hold.
- `exit_pad_blocks_transition_during_teleport` — same spawn fix; teleport gate still preserves state as expected.
- `chrono_miner_archive_cleared_after_undock_picks_new_target` — same spawn fix; (15, 11) ore is still within local_continuation_radius from (11, 12), so the test logic works unchanged.

The tests that DON'T touch the exit cell — `dock_unloading_phase_awards_credits` (line 1101), `dock_exit_returns_to_search_ore` (line 1136), `dock_sequence_progresses_through_phases` (line 1003), and the 3 reachability tests on dev (lines 1338, 1385, 1432) — should continue to pass without changes.

If any test fails unexpectedly, READ the failure carefully and document before proceeding.

**Step 6: Commit**

```
git add src/sim/miner/miner_dock_sequence.rs src/sim/miner/miner_tests.rs
git commit -m "miner: dock-drive direct moves bypass foundation walkability

Five coordinated changes:

- refinery_exit_cell Y formula now matches the original engine's
  exit position (south-edge interior cell, e.g. (rx+1, ry+2) for 4x3
  refinery, was (rx+1, ry+1)).
- phase_exit_pad sets bypass_grid=true on the direct_move so the
  harvester can step through PathGrid-blocked foundation cells.
- EXIT_FACING constant removed: per Ghidra audit the 0x47 value
  was not a screen-facing. Letting the locomotor compute facing
  naturally from movement direction is correct.

Combined with the A* start-cell relaxation, this closes the
head-butt-after-unload bug."
```

---

### Task 7: Strip all DIAG[...] log lines

**Why:** Cleanup. The DIAG logs were throwaway diagnostic instrumentation added during the bug investigation. The fix is in; the logs no longer earn their keep. Order: anywhere after Task 6, but before Task 8 so the new test isn't drowned in DIAG noise.

**Files:**
- Modify: `src/sim/miner/miner_system.rs`
- Modify: `src/sim/miner/miner_dock_sequence.rs`

**Pattern:** Every DIAG line starts with the literal string `DIAG[`. Pure deletion — no replacement.

**Step 1: Find every DIAG line**

Grep the codebase for `DIAG\[`:

```
Use the Grep tool with pattern: DIAG\[
glob: src/sim/miner/*.rs
output_mode: content
-n: true
```

Expected hits (per the original prompt's inventory):
- `miner_system.rs` — DIAG[heartbeat] in `process_miner` (once per second per miner, full state log)
- `miner_system.rs` — DIAG[search_ore enter / *_picked / wait_no_ore] in `handle_search_ore`
- `miner_system.rs` — DIAG[move_to_ore no_target / depleted / wait_teleport / arrived / astar_issued / astar_post / direct_move_issued / direct_move_post]
- `miner_dock_sequence.rs` — DIAG[exit_pad_arrival] in `phase_exit_pad`

**Step 2: Delete each `log::info!` block that prints a DIAG line**

For each DIAG hit, delete the entire `log::info!(...)` macro invocation, including:
- The opening `log::info!(`
- The format string starting with `"DIAG[..."`
- All argument lines
- The closing `);`
- Any leading comment that says `// DIAG: ... Remove after diagnosis.`

Do NOT delete surrounding code — only the diagnostic logging.

For multi-line `log::info!(...)` macros, ensure the entire macro is removed including any trailing newline.

**Step 3: Verify no DIAG references remain**

Run the same grep again:

```
Grep tool with pattern: DIAG\[
glob: src/sim/miner/*.rs
```

Expected: no results.

Also grep for the comment markers:

```
Grep tool with pattern: DIAG:
glob: src/sim/miner/*.rs
```

Expected: no results.

**Step 4: Verify build and tests**

Run: `cargo check --workspace`
Expected: PASS.

Run: `cargo clippy --all-targets`
Expected: PASS, no new warnings. (If `log` was the only `use` for some module, you may need to remove a now-unused `use log::info;` or similar import — clippy will flag it.)

Run: `cargo test miner`
Expected: PASS — removing logs doesn't affect test logic.

**Step 5: Commit**

```
git add src/sim/miner/miner_system.rs src/sim/miner/miner_dock_sequence.rs
git commit -m "miner: strip DIAG[...] logs added during head-butt diagnosis

The fix is in; the throwaway diagnostic instrumentation no longer
earns its keep. Removes ~10 log::info! calls from process_miner,
handle_search_ore, handle_move_to_ore, and phase_exit_pad."
```

---

### Task 8: Add `test_harvester_undocks_through_foundation_to_outside_ore`

**Why:** Pins the end-to-end fix with a structural test. Verifies the harvester can drive from the pad through the foundation to the exit cell, then SearchOre + A* finds a path out to ore. Order: last — depends on Tasks 1-6 all being in place.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs` — add a new test function, follow the existing test-construction pattern in the file.

**Pattern:** Existing miner tests in this file (e.g. `harvester_on_tiberium_falls_back_to_neighbor_zone`, `reachable_ore_picked_over_closer_unreachable`) for entity construction, simulation setup, and tick-loop assertions.

**Step 1: Understand why a new ticking helper is needed**

The existing `tick_miners_n` helper at `miner_tests.rs:218` calls `crate::sim::movement::tick_movement(...)` — the lightweight version that passes `path_grid: None` to `tick_movement_with_grid`. With `path_grid` None, the walkability check at `movement_step.rs:446` short-circuits to `true` (because `path_grid.map_or(true, ...)` is `true` when `path_grid` is `None`). That means the existing test infrastructure CAN'T exercise the `bypass_grid` codepath at all — every cell is "walkable" in tests regardless of foundation status.

Our new test must use `tick_movement_with_grid` (the full version) with a real `PathGrid` that has the GAREFN footprint marked blocked via `crate::sim::pathfinding::PathGrid::block_building_footprint`. Otherwise the test would pass even WITHOUT the `bypass_grid` change — meaningless coverage.

**Step 2: Identify the helpers the test will use**

From reading `miner_tests.rs`:
- `miner_rules() -> RuleSet` (line 25) — has HARV, CMIN, GAREFN(4x3) defined
- `spawn_miner(sim, sid, kind, rx, ry) -> u64` (line 98) — pass `MinerKind::War` for HARV
- `spawn_refinery(sim, sid, rx, ry)` (line 138) — places a GAREFN
- `place_ore(sim, rx, ry, amount: u16)` (line 191) — adds ore at a cell
- `MinerConfig::default()` — standard config
- `get_miner(sim, entity_id) -> Miner` (line 237) — read miner state

There is NO existing `place_harvester` (use `spawn_miner` with `MinerKind::War`). There is NO existing helper that builds a path_grid with footprints blocked — the test will construct one inline.

**Step 3: Add the new test**

Append at the end of `src/sim/miner/miner_tests.rs`:

```rust
/// End-to-end pin for the head-butt-after-unload fix. Exercises the full
/// chain: phase_exit_pad's bypass_grid drive → arrival → SearchOre → A*
/// from a blocked-start cell → MoveToOre. Uses a real PathGrid with the
/// refinery foundation blocked, so the test would FAIL without the
/// bypass_grid wiring (Task 6) and the A* start-relaxation (Task 4).
#[test]
fn harvester_undocks_through_foundation_to_outside_ore() {
    use crate::sim::pathfinding::PathGrid;

    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    // 4x3 GAREFN at (10, 10) — foundation occupies (10..=13, 10..=12).
    spawn_refinery(&mut sim, 100, 10, 10);

    // Place an ore patch at (11, 14) — south of the foundation, reachable
    // once the harvester clears the south edge.
    place_ore(&mut sim, 11, 14, 1200);

    // Build a 32x32 path_grid and mark the GAREFN footprint blocked.
    // This is the critical setup that makes the test meaningful — without
    // the blocked footprint, movement_step's walkability check would
    // succeed regardless of bypass_grid.
    let mut path_grid = PathGrid::new(32, 32);
    path_grid.block_building_footprint(10, 10, "4x3");

    // Spawn the harvester at the dock pad (13, 11) with cargo emptied
    // and dock state set to ExitPad — simulates "just finished unloading".
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 13, 11);
    {
        let entity = sim.entities.get_mut(miner_id).expect("harvester entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.cargo.clear();
        miner.state = MinerState::Dock;
        miner.dock_phase = RefineryDockPhase::ExitPad;
        miner.reserved_refinery = Some(100);
    }
    sim.production.dock_reservations.try_reserve(100, miner_id);

    // Tick the full pipeline: miner state machine + movement with the
    // blocked-footprint path_grid. Use enough ticks for the full sequence
    // (drive to exit + arrival + SearchOre + A* + drive south toward ore).
    // 60 ticks is comfortable headroom at speed=4 (~25 ticks per cell).
    use std::collections::BTreeMap;
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::occupancy::OccupancyGrid;
    use crate::sim::rng::SimRng;
    let alliances = HouseAllianceMap::new();
    let terrain_costs = BTreeMap::new();
    let mut occupancy = OccupancyGrid::new();
    let mut rng = SimRng::new(0);

    for _tick in 0..60 {
        crate::sim::miner::miner_system::tick_miners(
            &mut sim, &rules, &config, Some(&path_grid),
        );
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

    // Assertions on the final state.
    let entity = sim.entities.get(miner_id).expect("harvester still alive");
    let miner = entity.miner.as_ref().expect("miner component");

    // (1) Harvester transitioned out of Dock state — phase_exit_pad
    //     reached the arrival branch and ran cleanup.
    assert_ne!(
        miner.state,
        MinerState::Dock,
        "harvester should have transitioned out of Dock; pos=({},{}), state={:?}",
        entity.position.rx, entity.position.ry, miner.state,
    );

    // (2) phase_exit_pad cleared the dock reservation on arrival.
    assert!(
        miner.reserved_refinery.is_none(),
        "phase_exit_pad should have cleared reserved_refinery; got {:?}",
        miner.reserved_refinery,
    );

    // (3) Harvester either escaped the foundation south edge OR is targeting
    //     the ore patch — both prove SearchOre + A* succeeded from the
    //     (formerly blocked) start cell. If the harvester is still at
    //     (11, 12) with no ore target, A* failed silently — the relaxation
    //     in Task 4 isn't doing its job.
    let escaped_foundation = entity.position.ry > 12 || entity.position.rx < 10 || entity.position.rx > 13;
    let targeting_ore = miner.target_ore_cell == Some((11, 14));
    assert!(
        escaped_foundation || targeting_ore,
        "harvester should have escaped or be targeting ore; pos=({},{}), target_ore={:?}, state={:?}",
        entity.position.rx, entity.position.ry, miner.target_ore_cell, miner.state,
    );
}
```

**Note on the imports inside the function:** they're scoped narrowly because the imports aren't needed by other tests in the file. If `cargo clippy` flags this style, hoist them to the top-of-file imports. Read the file's existing import style and match it.

**Step 3: Verify**

Run: `cargo test miner::miner_tests::harvester_undocks_through_foundation_to_outside_ore -- --nocapture`
Expected: PASS.

If it fails, the failure message + the test's diagnostic asserts (`pos=({},{}), target={:?}`) should pinpoint where in the chain the harvester got stuck — useful for diagnosis without re-adding DIAG logs.

**Step 4: Run the full miner test suite to confirm no regressions**

Run: `cargo test miner`
Expected: ALL PASS.

**Step 5: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add harvester_undocks_through_foundation_to_outside_ore

End-to-end pin for the head-butt-after-unload fix. Verifies the
harvester drives through PathGrid-blocked foundation cells via
bypass_grid, transitions to SearchOre on arrival, and the A*
ore search succeeds from the (now-blocked) inside-foundation
start cell."
```

---

### Task 9: Final integration verification

**Why:** Confirms the full plan works as a unit. Order: after all prior tasks committed.

**Files:** none modified.

**Step 1: Format + lint + full test suite**

Run sequentially (each must pass before the next):

```
cargo fmt --all
cargo clippy --all-targets
cargo test --workspace
```

Expected for each: PASS.

If `cargo fmt` modifies files, commit the formatting change separately:

```
git add -u
git commit -m "fmt: cargo fmt after undock fix"
```

**Step 2: Targeted test inventory**

Run the following test groups individually to confirm coverage:

```
cargo test pathfinding
cargo test movement
cargo test miner
```

Expected: ALL PASS.

Specific tests to confirm appear in PASS output:
- `test_find_path_blocked_start_finds_path_through_walkable_neighbors` (new, Task 5)
- `test_find_path_blocked_start_all_neighbors_blocked_returns_none` (new, Task 5)
- `harvester_undocks_through_foundation_to_outside_ore` (new, Task 8)
- `harvester_on_tiberium_falls_back_to_neighbor_zone` (existing, must still pass)
- `reachable_ore_picked_over_closer_unreachable` (existing, must still pass)
- `unreachable_ore_filtered_out` (existing, must still pass)
- `exit_pad_clears_ore_targets_on_arrival` (existing, must still pass — verifies the preserved arrival branch)
- `exit_pad_blocks_transition_during_teleport` (existing, must still pass — verifies the preserved teleport gate)

The OLD `test_find_path_blocked_start` should NOT appear in the output (it was renamed in Task 5).

**Step 3: Manual sanity check (no implementation)**

Open `src/sim/miner/miner_system.rs` and `src/sim/miner/miner_dock_sequence.rs` and grep for `DIAG[`. Confirm zero matches. (The "workaround comment" the design doc mentions about `direct_move` bypassing passability for ore approach is now at `miner_system.rs:436-441` (the `// Adjacent to ore? The passability matrix blocks Tiberium ...` block) and `:467-470` (the `// After issuing the A* move, mark it as ignore_terrain_cost ...` block). Verify both are still present, untouched. They're independent of this fix — they handle ore-tile traversal via `ignore_terrain_cost`, not foundation traversal via `bypass_grid`.)

**Step 4: Branch state check**

Run: `git log --oneline dev..HEAD` (or just `git log --oneline -10`) and confirm the commit list looks like:

1. `movement: add bypass_grid flag to MovementTarget`
2. `movement: gate path_grid walkability on bypass_grid`
3. `movement: reset bypass_grid in segment-replan rebuild`
4. `pathfinding: allow A* to start from a blocked cell`
5. `pathfinding: pin new blocked-start A* contract via tests`
6. `miner: dock-drive direct moves bypass foundation walkability` (covers exit-cell formula, both phase_rotate_to_pad and phase_exit_pad bypass_grid wiring, EXIT_FACING removal, AND the 4 test updates in miner_tests.rs)
7. `miner: strip DIAG[...] logs added during head-butt diagnosis`
8. `miner_tests: add harvester_undocks_through_foundation_to_outside_ore`
9. (possibly) `fmt: cargo fmt after undock fix`

Each commit is small and revertable. The PR (when raised) is the diff between dev's pre-fix state and HEAD.

**Step 5: No commit needed for this task** — purely verification.

---

## Sources & References

- **Design doc:** [docs/plans/2026-04-27-refinery-undock-bypass-grid-design.md](2026-04-27-refinery-undock-bypass-grid-design.md)
- **Ghidra audit performed during brainstorm:**
  - `BuildingClass::UndockUnit` @ `0x004593A0` — verified call chain
  - `BuildingClass::GetCoords` @ `0x00447ac0` — verified vtable+0x48 dispatch and formula `coord = origin + (w-1, h-1) * 128`
  - `BuildingClass::GetRenderCoords` @ `0x00459ef0` — verified `origin = cell-center`
  - `DriveLocomotionClass::Force_Track` @ `0x004B0C40` — verified vtable+0x70 dispatch; first arg stored at locomotor field +0x54 (purpose unverified, NOT a screen-facing)
  - `vtable_BuildingClass` at `0x007e3ebc` — confirmed slot layout
- **Ghidra reports cited:**
  - `ra2-rust-game-docs/HARVESTER_DOCK_UNLOAD.md` (audited; §4 errors documented in design doc)
  - `ra2-rust-game-docs/WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` (audited; §6 disagrees with truth)
  - `ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md` §4.1 (verifies A* has no start-cell passability check)
  - `ra2-rust-game-docs/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` §State 0 (verifies post-undock flow)
- **INI keys:** none new (uses existing foundation dimensions parsed from `art(md).ini`)
- **Related code (existing patterns mirrored):**
  - `src/sim/components.rs:256` — `ignore_terrain_cost` flag pattern (mirrored by `bypass_grid`)
  - `src/sim/movement/movement_step.rs:448-450` — short-circuit gate pattern (mirrored at line 446)
  - `src/sim/movement/movement_tick.rs:221` — segment-replan rebuild reset pattern
  - `src/sim/miner/miner_tests.rs` — existing reachability-test patterns (mirrored in Task 8)
- **Related prior commits on dev branch:**
  - `d45a225 miner: filter ore-search candidates by zone-based reachability` (G1 fix; complements this work)
  - `e9babfe`, `27eb9c1`, `9493a00` (G1 test suite)
  - `037d572 miner: clear stale ore targets and add teleport gate on undock` (the prior partial fix that the head-butt-bug investigation found insufficient; this plan is the actual fix)
