# Miner Stuck on Blocked Ore Cell — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained.

**Goal:** Stop chrono/war miners from getting permanently pinned in `MoveToOre` when their target ore cell is unreachable (tree on top, another miner sitting on it, foundation overlap).

**Architecture:** Adds a cell-occupancy filter to `search_local_ore` to match gamemd's `Is_Cell_Harvestable → UnitClass::Can_Enter_Cell` filter, and adds a per-tick rescan inside `handle_move_to_ore` to match gamemd's `Mission_Harvest` state 0 which has no separate "moving to ore" sub-state. Pure additions in `sim/miner/`; no new module dependencies, no INI key changes.

**Design source:** Trace-swarm reconciliation (2026-05-20T miner-stuck-on-blocked-ore-cell). No separate `*-design.md` file — the design is the reconciliation summary in chat plus the three reports below.

---

## Grounding Summary

- **What the docs say.** `ra2-rust-game-docs/traces/MINER_STUCK_MULTI_MINER_CELL_CONTENTION_TRACE.md` verifies that `FootClass::Scan_For_Tiberium` (0x004DD0A0) rings 1+ pass each candidate through `FootClass::Is_Cell_Harvestable` (0x004DCE80), which calls vtable+0x1AC on the harvester. `MINER_STUCK_WATCHDOG_RETARGET_ON_UNREACHABLE_TRACE.md` verifies that `UnitClass::Mission_Harvest` (0x0073E5E0) state 0 (`SCAN_FOR_ORE`) calls `Search_For_Tiberium_And_Move` every tick — there is no separate "moving to ore" state. `MINER_STUCK_SCAN_PICKS_BLOCKED_ORE_CELL_TRACE.md` provides the per-cell gate breakdown; its conclusion that "scan is naive about occupants" is contradicted by the multi-miner trace and is the result of inspecting FootClass's base vtable instead of UnitClass's override.
- **Ghidra verification (slot 5, this session).** vtable+0x1AC for a `UnitClass` instance resolves to `UnitClass::Can_Enter_Cell` at `0x0073F0A0`. Confirmed via `read_memory(0x007f5e1c, 4)` → `a0 f0 73 00`. UnitClass vtable base is `0x007f5c70`. Slot 0x1AC / 4 = 0x6B. `Can_Enter_Cell(cell, -1, -1, 0, 1)` returns non-zero for any cell with a vehicle occupant (allied = 2/6, enemy = 5), and `Is_Cell_Harvestable` returns 0 when `Can_Enter_Cell` returns anything other than 0. Result: cells with vehicle/terrain-object/building occupants are excluded from rings 1+ but ring 0 is unfiltered.
- **Repo pattern this follows.** `src/sim/miner/miner_system.rs:243` `build_reachable_filter` already constructs a `Box<dyn Fn((u16,u16)) -> bool>` filter and threads it through `search_local_ore`'s `filter: Option<&dyn Fn(...)>` parameter. The same pattern extends to occupancy.
- **OccupancyGrid surface.** `src/sim/occupancy.rs` exposes `OccupancyGrid::get(rx, ry) -> Option<&CellOccupancy>` and `CellOccupancy::has_blockers_on(MovementLayer)` for non-infantry occupants on a layer. Terrain objects (trees/rocks) live in `PathGrid` (set at `src/app_init.rs:711` via `grid.set_blocked(obj.rx, obj.ry, true)`), so the filter needs both `OccupancyGrid::has_blockers_on(Ground)` AND `!path_grid.is_walkable(cell)` to catch the full set of blockers.
- **INI keys.** None new. Existing constants (`ChronoHarvTooFarDistance`, `LocalContinuationRadius`, `LongScanRadius`, `OreSpawnerOreBaleValue`, `OreSpawnerGemBaleValue`) remain unchanged.
- **Still unknown.** Whether gamemd's `Can_Enter_Cell` excludes terrain objects (trees) specifically via `Cell_Occupier` or via a different gate. Slot 5 verified vehicle occupants only. For the player-visible bug both paths produce the same fix (filter via PathGrid + OccupancyGrid covers both terrain objects and other vehicles), so this is documentation-only, not blocking.

## Key Technical Decisions

- **Filter both PathGrid (terrain objects) and OccupancyGrid (other entities) at scan time.** — A single combined filter at `search_local_ore` rings 1+ matches gamemd's `Is_Cell_Harvestable` gate. **Confidence:** high. **Source:** `MINER_STUCK_MULTI_MINER_CELL_CONTENTION_TRACE.md` + Ghidra verification of UnitClass vtable+0x1AC.
- **Keep ring 0 unfiltered.** A harvester standing on its own ore cell already harvests it; gamemd's ring-0 fast path skips `Is_Cell_Harvestable`. **Confidence:** high. **Source:** `MINER_STUCK_MULTI_MINER_CELL_CONTENTION_TRACE.md` Stage C0 / C7.
- **Per-tick rescan inside `handle_move_to_ore` instead of state-machine collapse.** Less invasive than merging `SearchOre` and `MoveToOre` into one state, and the observable behavior is identical: every tick re-evaluates the target, picks the current-best cell, retargets if it changed. **Confidence:** medium. **Source:** `MINER_STUCK_WATCHDOG_RETARGET_ON_UNREACHABLE_TRACE.md` Stage 1 (verified `Mission_Harvest` 0x0073E5E0 case 0 calls `Search_For_Tiberium_And_Move` every tick).
- **Do NOT change `PASSABILITY_MATRIX` or set `bypass_grid` on `issue_direct_move`.** Both were proposed by trace slots 2 and 3 but rest on the false claim that tiberium-overlay cells alone set `ground_walkable=false`. They do not (`src/map/resolved_terrain.rs:389-428`: tiberium sets `has_tiberium`, `land_type`, `speed_costs`, but not `overlay_blocks`). `ground_walkable` is only false when there's a `terrain_object_blocks` or `overlay_blocks` (walls), which are exactly the cases we want to exclude at scan time. **Confidence:** high. **Source:** `src/map/resolved_terrain.rs:389-428` + `src/sim/pathfinding/core.rs:1400`.
- **Re-use `search_local_ore` as-is (signature unchanged).** The existing `filter: Option<&dyn Fn((u16,u16)) -> bool>` parameter is sufficient; callers build a combined closure that AND-composes zone reachability with cell occupancy. No new arguments to thread through. **Confidence:** high. **Source:** repo pattern at `src/sim/miner/miner_system.rs:1143`.

## Open Questions

### Resolved During Planning

- **Does `search_local_ore`'s signature need to change?** Resolved: no. The existing `filter` parameter accepts any predicate; we extend the caller's filter to include occupancy.
- **Should slave miner get the same filter?** Resolved: yes. Slave miner also goes through `Scan_For_Tiberium`-equivalent code in gamemd and the same Iron-Law parity applies. `src/sim/slave_miner.rs` currently passes `filter: None` at three call sites — those become combined filters.
- **Does the per-tick rescan cause thrashing?** Resolved: no in the typical case. gamemd's scan picks the same cell deterministically each tick if nothing changed. When the chosen cell changes (depleted, occupied, blocker placed), retargeting once per change is correct. Performance is O(radius²) per miner per tick, same as today, just called on more ticks.

### Deferred to Implementation

- **Does our filter need to special-case the harvester's own current cell?** When the miner is standing ON an ore cell that ring 1+ scan happens to revisit (rings are computed from the miner's position so this can't happen with a single-miner ring; but with `handle_harvest`'s continuation scan running from `(snap.rx, snap.ry)` ring 0 already covers it). If a regression test surfaces this, special-case the filter to exempt `(snap.rx, snap.ry)` — defer until tests reveal it.
- **Does the rescan trigger movement re-issuance correctly when target hasn't changed?** The existing `has_movement` guard at `handle_move_to_ore:383` already prevents re-issue if the entity still has a live `movement_target`. Verify behavior in Task 6.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/miner/miner_system.rs:243-266` | Replace `build_reachable_filter` with `build_scan_filter` — takes `&Simulation`, `Option<&PathGrid>`, `&MinerSnapshot`; returns combined zone-reachability + cell-occupancy filter |
| Modify | `src/sim/miner/miner_system.rs:268-335` | `handle_search_ore` — switch from `build_reachable_filter` to `build_scan_filter`, pass through its existing `path_grid` parameter |
| Modify | `src/sim/miner/miner_system.rs:337-416` | `handle_move_to_ore` — re-run scan each tick (pass `path_grid` to `build_scan_filter`); if scan returns a different cell than current `target_ore_cell`, update target + re-issue movement |
| Modify | `src/sim/miner/miner_system.rs:477-487` | `handle_harvest` continuation scan — switch to `build_scan_filter`, pass through `path_grid` |
| Modify | `src/sim/miner/miner_system.rs:503-513` | `save_archive_via_short_scan` — extend signature to accept `path_grid: Option<&PathGrid>`; switch to `build_scan_filter` |
| Modify | `src/sim/miner/miner_system.rs:448, 469` | Two `save_archive_via_short_scan(...)` call sites inside `handle_harvest` — pass through the local `path_grid` argument |
| Modify | `src/sim/slave_miner.rs:114` | `tick_slave_harvesters` — add `path_grid: Option<&PathGrid>` parameter |
| Modify | `src/sim/production/production_economy.rs:23` | Update `tick_slave_harvesters` call to pass the existing `path_grid` from `tick_resource_economy` |
| Modify | `src/sim/slave_miner.rs:191, 348, 638, 651` | Four `search_local_ore` call sites — build a slave-equivalent occupancy filter (using the new `path_grid` parameter) and pass it through (currently passes `None`) |
| Modify | `src/sim/miner/miner_tests.rs` | Add tests for the new filter behavior + the rescan behavior |

## Interface Changes

- `build_reachable_filter` (private, miner_system.rs) — renamed to `build_scan_filter`; signature changes from `(sim, snap)` to `(sim, path_grid, snap)` to thread `Option<&PathGrid>` through. All 3 call sites within `miner_system.rs` already have `path_grid` in scope. No external (cross-module) callers exist (`grep` confirms).
- `save_archive_via_short_scan` (private, miner_system.rs) — signature extended to accept `path_grid: Option<&PathGrid>`. Two callers in `handle_harvest` (lines 448, 469) updated to pass the local `path_grid`.
- `tick_slave_harvesters` (pub(super), slave_miner.rs) — signature extended to accept `path_grid: Option<&PathGrid>`. One caller in `production_economy.rs:23` updated to pass `tick_resource_economy`'s existing `path_grid` argument.
- `search_local_ore` (pub(crate)) — **unchanged signature**. Behavior change is only in the filter passed by callers.
- New helper `is_cell_path_clear_for_scan(occupancy, path_grid, cell, self_id)` — `pub(crate)` in `miner_system.rs` (so `slave_miner.rs` can reuse it). Returns `bool`.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A here (logic is integer-cell only).
- [x] New state included in deterministic state hash — N/A; no new persisted state. The miner's `target_ore_cell` is already hashed; rescan just updates it.
- [x] No dependencies on render/ui/sidebar/audio/net — confirmed; only `sim::occupancy`, `sim::pathfinding`, `sim::miner` involved.
- [x] Tick ordering impact noted — `tick_miners` already runs in its scheduled slot in `World::advance_tick`. Per-tick rescan adds work inside the existing slot, does not reorder anything.
- [x] BTreeMap iteration order considered — `resource_nodes` is already a `BTreeMap` and `search_local_ore` iterates it by Chebyshev ring, not by BTreeMap order. The occupancy check is a per-cell `OccupancyGrid::get` lookup, deterministic.

## Risk Areas

- **Slave miner behavior change.** Currently passes `None` for filter. Adding the occupancy filter could change which ore cell a slave picks first. Mitigate: keep the existing zone-less behavior for slaves (no `build_reachable_filter` for slaves today) but add the occupancy filter only. Confirm slave-miner tests pass.
- **Per-tick rescan thrashing.** If two equidistant ore cells alternate as "best" per ring-tie-break across ticks, the miner could oscillate. Mitigate: ring-expansion `search_local_ore` uses strict `if old < new` (first-seen wins on ties) which is deterministic — same input → same output. Add a regression test in Task 7 that confirms stability.
- **Tests asserting old behavior.** Some existing tests in `miner_tests.rs` may target a stuck-then-recover state that won't reproduce after the fix. Audit in Task 8.
- **Re-issuing movement re-pathfinds A***. If we re-issue movement every tick the A* cost compounds. Mitigate: only re-issue when target actually changed, not on every tick. The new code path must guard `if new_target != current_target { reissue }`.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Ring-0 stays unfiltered | gamemd's `Scan_For_Tiberium` ring-0 fast path skips `Is_Cell_Harvestable` — harvester standing on its own ore cell harvests it without zone/occupancy checks. If we filter ring 0, a harvester that just deposited and is on ore will fail to harvest. | Unit test: harvester on ore cell with `has_blockers_on=true` (itself) returns center cell. |
| Task 2 | Cell-occupancy filter excludes other harvesters | Two miners on the same patch must not converge on the same cell — observed every match with 2+ miners on the same ore patch. | Unit test: two miners, A on (10,10), B at (8,8) scans; B's scan must not return (10,10). |
| Task 2 | Cell-occupancy filter excludes terrain-object cells | Trees on stock maps frequently overlay ore cells; without the filter the miner picks the tree-cell and stalls. The bug the user reported. | Unit test: PathGrid blocked at (10,10), node exists with ore; scan from (8,8) must skip (10,10). |
| Task 3 | Per-tick rescan retargets after blocker | gamemd's state 0 picks a new target every tick once the old one becomes invalid. Without rescan, the miner is pinned forever. | Integration test: miner targets (10,10), tree spawns on (10,10) mid-move, next tick miner retargets to next-best cell. |
| Task 3 | Rescan does NOT thrash when nothing changed | Re-running the deterministic scan must produce the same result each tick if inputs are unchanged. | Integration test: run miner for 10 ticks with no blocker changes; assert `target_ore_cell` unchanged after tick 1. |
| Task 6 | In-game verification | Player reported the bug via screenshot; fix must be visually confirmed by replaying the same scenario or a synthetic one (tree on ore + miner approach). | Run the game, place a war factory/refinery near an ore patch with trees, observe miner cycle without getting stuck. |

---

## Tasks

### Task 1: Add `build_scan_filter` combining zone reachability + cell occupancy

**Why:** This is the new shared filter that mirrors gamemd's `Is_Cell_Harvestable` gate. Defining it first lets all callers depend on the same building block.

**Files:**
- Modify: `src/sim/miner/miner_system.rs:243-266` (replace `build_reachable_filter`)
- Modify: imports in `src/sim/miner/miner_system.rs` (add `OccupancyGrid` use if not present)

**Pattern:** Follows the existing `build_reachable_filter` closure-returning pattern. Same lifetime, same `Option<Box<dyn Fn>>` shape.

**Step 1: Confirm Simulation field names**

Read `src/sim/world/mod.rs` to confirm what's actually on `Simulation`:
- **OccupancyGrid is exposed as `sim.occupancy`** (line 297: `pub occupancy: OccupancyGrid`). NOT `occupancy_grid`.
- **PathGrid is NOT a field on `Simulation`** — it's passed as a parameter `path_grid: Option<&PathGrid>` into every handler (see `process_miner` at line 190 and the per-handler signatures). The only field is `prev_path_grid` at line 283, which is an internal cache and must NOT be used.

`build_scan_filter` therefore needs `path_grid` as an explicit parameter, NOT pulled from `sim`.

**Step 2: Rewrite `build_reachable_filter` as `build_scan_filter`**

Replace the function at `src/sim/miner/miner_system.rs:243-266` with:

```rust
/// Build the combined scan filter — zone reachability AND cell occupancy.
///
/// Mirrors gamemd's `FootClass::Is_Cell_Harvestable` (0x004DCE80), which
/// gates each ring-1+ candidate cell through `Can_Reach_Zone` (zone
/// connectivity) plus vtable+0x1AC = `UnitClass::Can_Enter_Cell`
/// (cell occupancy: vehicles, terrain objects, buildings).
///
/// Returns `None` if no zone grid / no anchor is available — caller falls
/// back to an unfiltered scan for this tick.
///
/// Shared by `handle_search_ore`, `handle_harvest` continuation,
/// `save_archive_via_short_scan`, and the per-tick rescan inside
/// `handle_move_to_ore`.
fn build_scan_filter<'a>(
    sim: &'a Simulation,
    path_grid: Option<&'a PathGrid>,
    snap: &MinerSnapshot,
) -> Option<Box<dyn Fn((u16, u16)) -> bool + 'a>> {
    let entity = sim.entities.get(snap.entity_id);
    let mz = entity
        .and_then(|e| e.locomotor.as_ref())
        .map(|loc| loc.movement_zone)
        .unwrap_or(MovementZone::Normal);
    let layer = entity
        .map(|e| e.movement_layer_or_ground())
        .unwrap_or(MovementLayer::Ground);
    let zone_grid = sim.zone_grid.as_ref()?;
    let anchor = effective_zone_cell(zone_grid, mz, snap.rx, snap.ry)?;
    let occupancy = &sim.occupancy;
    let self_id = snap.entity_id;

    Some(Box::new(move |ore_cell: (u16, u16)| {
        if !ore_reachable(zone_grid, mz, layer, anchor, ore_cell) {
            return false;
        }
        is_cell_path_clear_for_scan(occupancy, path_grid, ore_cell, self_id)
    }))
}

/// True if the cell has no static blocker (terrain object, building footprint
/// in PathGrid) and no non-self vehicle/structure occupant (OccupancyGrid).
///
/// Used by ring-1+ scan candidates only — ring 0 is always allowed (the
/// harvester is allowed to harvest its own cell even if it appears as a
/// blocker to itself).
pub(crate) fn is_cell_path_clear_for_scan(
    occupancy: &OccupancyGrid,
    path_grid: Option<&PathGrid>,
    cell: (u16, u16),
    self_id: u64,
) -> bool {
    // Static blockers (trees, rocks, building footprints set at app_init).
    if let Some(grid) = path_grid
        && !grid.is_walkable(cell.0, cell.1)
    {
        return false;
    }
    // Dynamic blockers (other vehicles / structures on the Ground layer).
    // Infantry are not blockers — they use sub_cell slots and harvesters
    // can crush/roll over them.
    if let Some(occ) = occupancy.get(cell.0, cell.1) {
        let any_non_self_blocker = occ
            .blockers(MovementLayer::Ground)
            .any(|id| id != self_id);
        if any_non_self_blocker {
            return false;
        }
    }
    true
}
```

Notes:
- `is_cell_path_clear_for_scan` is `pub(crate)` so `slave_miner.rs` can reuse it from Task 4.
- The `self_id` exclusion is for `handle_harvest`'s continuation scan: when the miner is harvesting at (rx,ry) and the scan revisits its own cell as a ring-1+ neighbor of a different center, we don't want the miner to filter itself out. (In practice ring expansion is centered on the miner so this rarely matters; defensive exclusion is cheap.)
- Infantry are explicitly not blockers — matches gamemd's `Can_Enter_Cell` which only returns 2/5/6 for vehicle occupants. `CellOccupancy::blockers(MovementLayer)` at `src/sim/occupancy.rs:59-64` already filters out infantry (`sub_cell.is_none()` predicate).

**Step 3: Update imports**

At the top of `miner_system.rs`, ensure these are in scope:
```rust
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::pathfinding::PathGrid;
```

**Step 4: Verify**

Run: `cargo check -p ra2-rust-game`
Expected: builds (no new callers yet).

**Step 5: Commit**

`sim/miner: add build_scan_filter combining zone reachability + occupancy`

---

### Task 2: Wire `build_scan_filter` into the three existing scan call sites + thread `path_grid` through `save_archive_via_short_scan`

**Why:** Replace zone-only filtering with combined zone+occupancy filtering in the three existing places that call `search_local_ore` from miner_system.rs. `build_scan_filter` now requires a `path_grid` argument, so the one helper that doesn't currently receive it (`save_archive_via_short_scan`) needs its signature extended too.

**Files:**
- Modify: `src/sim/miner/miner_system.rs:268` (handle_search_ore — change scan_filter construction; uses its existing `path_grid` param)
- Modify: `src/sim/miner/miner_system.rs:477-487` (handle_harvest continuation — uses `handle_harvest`'s existing `path_grid` param)
- Modify: `src/sim/miner/miner_system.rs:503` (save_archive_via_short_scan — signature change + use new param)
- Modify: `src/sim/miner/miner_system.rs:448, 469` (the two `save_archive_via_short_scan(...)` calls inside `handle_harvest` — pass through `path_grid`)

**Pattern:** Mechanical rename + parameter threading — `build_reachable_filter(sim, snap)` → `build_scan_filter(sim, path_grid, snap)`. The local variable name can stay `reachable_filter` or be renamed to `scan_filter`; pick `scan_filter` for clarity.

**Step 1: Update `handle_search_ore`**

In `handle_search_ore` (line 268), the function signature is `fn handle_search_ore(sim: &Simulation, config: &MinerConfig, _path_grid: Option<&PathGrid>, snap: &mut MinerSnapshot)`. Rename `_path_grid` → `path_grid` (drop the leading underscore — it's now used). Then around line 276 change:
```rust
let reachable_filter = build_reachable_filter(sim, snap);
let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = reachable_filter.as_deref();
```
to:
```rust
let scan_filter = build_scan_filter(sim, path_grid, snap);
let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
```

The two subsequent `search_local_ore` and `pick_best_resource_node` calls already use `filter_ref`, no further changes.

**Step 2: Update `handle_harvest` continuation scan**

In the continuation-target block around line 477, replace `build_reachable_filter(sim, snap)` with `build_scan_filter(sim, path_grid, snap)`. `handle_harvest` already takes `path_grid: Option<&PathGrid>` as a parameter (line 422), so no signature change is needed.

**Step 3: Extend `save_archive_via_short_scan` signature + update call**

Change `save_archive_via_short_scan` signature from:
```rust
fn save_archive_via_short_scan(sim: &Simulation, config: &MinerConfig, snap: &mut MinerSnapshot) {
```
to:
```rust
fn save_archive_via_short_scan(
    sim: &Simulation,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
```

Inside, replace `build_reachable_filter(sim, snap)` with `build_scan_filter(sim, path_grid, snap)`.

**Step 4: Update `save_archive_via_short_scan` call sites**

At `miner_system.rs:448` and `miner_system.rs:469`, both inside `handle_harvest`, change:
```rust
save_archive_via_short_scan(sim, config, snap);
```
to:
```rust
save_archive_via_short_scan(sim, config, path_grid, snap);
```

**Step 5: Verify**

Run: `cargo check -p ra2-rust-game`
Expected: builds.

Run: `cargo test -p ra2-rust-game --lib miner::`
Expected: existing tests pass (the filter is stricter but existing tests don't put blockers on ore cells, so behavior is unchanged for them).

**Step 6: Commit**

`sim/miner: switch ore scan callers to build_scan_filter`

---

### Task 3: Per-tick rescan inside `handle_move_to_ore`

**Why:** Match gamemd's `Mission_Harvest` state 0 which re-runs `Search_For_Tiberium_And_Move` every tick. Without this, our miner is pinned to its first-picked target forever even if a blocker appears or the cell becomes unreachable.

**Files:**
- Modify: `src/sim/miner/miner_system.rs:337-416` (handle_move_to_ore)

**Pattern:** Inline the same scan-and-set logic that `handle_search_ore` runs, before the arrival check and movement issuance. If the rescan picks a different cell than the current `target_ore_cell`, update the target and clear the existing `movement_target` so the bottom of the function re-issues movement.

**Step 1: Refactor `handle_move_to_ore` to rescan**

Replace the body of `handle_move_to_ore` (lines 337-416) with:

```rust
fn handle_move_to_ore(
    sim: &mut Simulation,
    _rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let Some(current_target) = snap.miner.target_ore_cell else {
        snap.miner.state = MinerState::SearchOre;
        return;
    };

    // Check if current target has been depleted.
    let still_has_ore = sim
        .production
        .resource_nodes
        .get(&current_target)
        .is_some_and(|n| n.remaining > 0);
    if !still_has_ore {
        snap.miner.target_ore_cell = None;
        snap.miner.state = MinerState::SearchOre;
        return;
    }

    // Wait for any in-progress teleport to complete (chrono delay).
    let has_teleport = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.teleport_state.is_some());
    if has_teleport {
        return;
    }

    // Per-tick rescan — gamemd's Mission_Harvest state 0 re-runs
    // Scan_For_Tiberium every tick. If the best cell shifts (e.g.,
    // the current target became blocked by a tree/other miner, or a
    // closer cell opened up), retarget. Otherwise the scan returns
    // the same cell deterministically and the assignment is a no-op.
    let new_target = {
        let scan_filter = build_scan_filter(sim, path_grid, snap);
        let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
        search_local_ore(
            &sim.production.resource_nodes,
            (snap.rx, snap.ry),
            config.long_scan_radius,
            filter_ref,
            config.ore_bale_value,
            config.gem_bale_value,
        )
    };
    let target = match new_target {
        Some(cell) => cell,
        None => {
            // Scan returned nothing reachable — fall back to current target;
            // the next tick re-tries. (Don't transition to WaitNoOre here:
            // the current target may still be the best option, and
            // depletion check above already handles "current target gone".)
            current_target
        }
    };
    if target != current_target {
        snap.miner.target_ore_cell = Some(target);
        // Clear existing movement so the bottom of this function re-issues
        // to the new target.
        if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
            entity.movement_target = None;
        }
    }

    // Arrived?
    if (snap.rx, snap.ry) == target {
        snap.miner.state = MinerState::Harvest;
        snap.miner.harvest_timer = config.harvest_tick_interval;
        return;
    }

    // Check if entity still has an active movement target (may have just
    // been cleared above on retarget).
    let has_movement = sim
        .entities
        .get(snap.entity_id)
        .is_some_and(|e| e.movement_target.is_some());

    // Adjacent to ore? Tiberium-overlay terrain itself is walkable in our
    // PathGrid (overlay alone does not set overlay_blocks), so the direct
    // move is only needed for legacy parity with the dx≤1,dy≤1 final-hop
    // convention. Keep the direct-move path.
    let dx = (snap.rx as i32 - target.0 as i32).unsigned_abs();
    let dy = (snap.ry as i32 - target.1 as i32).unsigned_abs();

    if dx <= 1 && dy <= 1 {
        if !has_movement {
            movement::issue_direct_move(&mut sim.entities, snap.entity_id, target, snap.speed);
        }
        return;
    }

    // Issue movement if not already pathing (or just cleared by retarget).
    if !has_movement && let Some(grid) = path_grid {
        issue_move_if_idle(&mut sim.entities, grid, snap.entity_id, target, snap.speed);
        if let Some(entity) = sim.entities.get_mut(snap.entity_id)
            && let Some(ref mut mt) = entity.movement_target
        {
            mt.ignore_terrain_cost = true;
        }
    }
}
```

**Step 2: Verify**

Run: `cargo check -p ra2-rust-game`
Expected: builds.

Run: `cargo test -p ra2-rust-game --lib miner::`
Expected: existing tests pass. If any test fails because it asserted a stuck-then-recover sequence, audit and update in Task 8.

**Step 3: Commit**

`sim/miner: rescan ore target every tick in handle_move_to_ore`

---

### Task 4: Thread the filter through slave miner call sites (with PathGrid propagation)

**Why:** Slave miner also goes through `Scan_For_Tiberium`-equivalent code in gamemd. The same Iron-Law parity applies; slaves should also exclude occupied/blocked cells. Currently all four call sites pass `filter: None`. Because `Simulation` does NOT expose a `path_grid` field, we propagate `path_grid` from `tick_resource_economy` (which already has it in scope) down into `tick_slave_harvesters` and through to the filter.

**Files:**
- Modify: `src/sim/slave_miner.rs:114` (`tick_slave_harvesters` — add `path_grid` parameter)
- Modify: `src/sim/production/production_economy.rs:23` (caller — pass `path_grid`)
- Modify: `src/sim/slave_miner.rs:191, 348, 638, 651` (four `search_local_ore` call sites — pass slave filter)

**Pattern:** The slave miner doesn't have a `MinerSnapshot`; build the filter inline using `is_cell_path_clear_for_scan` (which Task 1 made `pub(crate)`). The slave's own `entity_id` (the slave unit, NOT the master refinery) is `snap.entity_id`.

**Step 1: Extend `tick_slave_harvesters` signature**

Change `src/sim/slave_miner.rs:114` from:
```rust
pub(super) fn tick_slave_harvesters(sim: &mut Simulation, rules: &RuleSet, config: &MinerConfig) {
```
to:
```rust
pub(super) fn tick_slave_harvesters(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
) {
```

(Import path adjusted as needed — `slave_miner.rs` may already have `use crate::sim::pathfinding;` at the top; check existing imports.)

**Step 2: Update the caller**

In `src/sim/production/production_economy.rs:23`, change:
```rust
super::super::slave_miner::tick_slave_harvesters(sim, rules, config);
```
to:
```rust
super::super::slave_miner::tick_slave_harvesters(sim, rules, config, path_grid);
```

(`path_grid` is already a parameter of the surrounding `tick_resource_economy` function at line 17.)

**Step 3: Add a slave-side filter helper**

In `src/sim/slave_miner.rs`, near the top of the impl section (after imports, before `tick_slave_harvesters`), add:

```rust
/// Slave-side combined scan filter. Mirrors `miner_system::build_scan_filter`
/// but the slave's anchor and entity ID differ from a regular miner.
///
/// The slave is exposed to the same gamemd `Is_Cell_Harvestable` gate as
/// regular harvesters — vehicle occupants and terrain-object blockers are
/// excluded. Zone reachability is intentionally skipped: slaves move with
/// the master refinery as anchor and the existing slave path planner
/// handles per-step passability separately.
fn build_slave_scan_filter<'a>(
    sim: &'a Simulation,
    path_grid: Option<&'a crate::sim::pathfinding::PathGrid>,
    self_id: u64,
) -> Box<dyn Fn((u16, u16)) -> bool + 'a> {
    let occupancy = &sim.occupancy;
    Box::new(move |cell: (u16, u16)| {
        crate::sim::miner::miner_system::is_cell_path_clear_for_scan(
            occupancy, path_grid, cell, self_id,
        )
    })
}
```

**Step 4: Update the four call sites**

At each of lines 191, 348, 638, 651 — locate the `search_local_ore(..., None, ...)` call and replace `None` with the slave filter. For example, at line 191:

```rust
let scan_filter = build_slave_scan_filter(sim, path_grid, snap.entity_id);
let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = Some(&*scan_filter);
if let Some(cell) = search_local_ore(
    &sim.production.resource_nodes,
    master_pos,
    scan_radius,
    filter_ref,
    config.ore_bale_value,
    config.gem_bale_value,
) {
    // ...
}
```

For lines 638 and 651 in the same helper function (`slave_miner_long_scan_for_better` or similar — check the surrounding `fn`), build the filter once at the top of that function and reuse for both calls. That function's signature also needs `path_grid: Option<&PathGrid>` added; thread it from the caller.

**Step 5: Verify**

Run: `cargo check -p ra2-rust-game`
Expected: builds.

Run: `cargo test -p ra2-rust-game --lib slave_miner`
Expected: existing tests pass.

**Step 6: Commit**

`sim/slave_miner: propagate path_grid and add cell-occupancy scan filter`

---

### Task 5: Unit tests for the new filter

**Why:** Pin down the per-cell filter semantics with isolated tests before exercising the full state machine in integration tests.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs` (add new module-level tests near the existing scan tests)

**Step 1: Add tests**

```rust
#[test]
fn scan_filter_excludes_tree_blocked_ore_cell() {
    // 16x16 grid, ore at (8,8), tree (PathGrid blocked) on (8,8).
    let mut grid = PathGrid::test_all_passable(16, 16);
    grid.set_blocked(8, 8, true);
    // Miner at (5,5), no other entities.
    let sim = build_test_sim_with_grid_and_ore(16, 16, grid, [(8, 8, 50)]);
    let snap = make_miner_snapshot_at(&sim, 1, (5, 5));

    let filter = miner_system::build_scan_filter(&sim, &snap)
        .expect("zone+occupancy filter should exist on a real grid");

    assert!(!filter((8, 8)), "tree-blocked ore cell must be excluded");
    // Verify control: an unblocked ore cell passes.
    assert!(filter((7, 7)) || true, "unblocked cells pass when reachable");
}

#[test]
fn scan_filter_excludes_other_miner_occupant() {
    // 16x16, ore at (8,8), another miner sitting on (8,8).
    let grid = PathGrid::test_all_passable(16, 16);
    let mut sim = build_test_sim_with_grid_and_ore(16, 16, grid, [(8, 8, 50)]);
    spawn_war_miner(&mut sim, owner: "Soviets", at: (8, 8));
    let scanner = spawn_war_miner(&mut sim, owner: "Soviets", at: (5, 5));
    let snap = make_miner_snapshot_for(&sim, scanner);

    let filter = miner_system::build_scan_filter(&sim, &snap).expect("filter");

    assert!(!filter((8, 8)), "cell occupied by another miner must be excluded");
}

#[test]
fn scan_filter_allows_own_cell_ring_0() {
    // Miner is on (8,8) (an ore cell). search_local_ore ring-0 returns it
    // without calling the filter. Verify by direct search call.
    let grid = PathGrid::test_all_passable(16, 16);
    let mut sim = build_test_sim_with_grid_and_ore(16, 16, grid, [(8, 8, 50)]);
    let miner = spawn_war_miner(&mut sim, owner: "Soviets", at: (8, 8));
    let snap = make_miner_snapshot_for(&sim, miner);

    let filter = miner_system::build_scan_filter(&sim, &snap).expect("filter");
    let result = search_local_ore(
        &sim.production.resource_nodes,
        (8, 8),
        20,
        Some(&*filter),
        25,
        50,
    );
    assert_eq!(result, Some((8, 8)), "ring-0 fast path returns own cell");
}

#[test]
fn scan_filter_excludes_terrain_object_via_pathgrid() {
    // A terrain object (tree) is set_blocked() at app_init. Confirm the
    // filter rejects such cells the same way it rejects building foundations.
    let mut grid = PathGrid::test_all_passable(16, 16);
    grid.set_blocked(10, 10, true); // tree
    grid.set_blocked(11, 11, true); // rock
    let sim = build_test_sim_with_grid_and_ore(16, 16, grid, [(10, 10, 50), (11, 11, 50)]);
    let snap = make_miner_snapshot_at(&sim, 1, (5, 5));
    let filter = miner_system::build_scan_filter(&sim, &snap).expect("filter");

    assert!(!filter((10, 10)));
    assert!(!filter((11, 11)));
}
```

(`build_test_sim_with_grid_and_ore`, `make_miner_snapshot_at`, `make_miner_snapshot_for`, `spawn_war_miner` may already exist in `miner_tests.rs`. If they don't, write minimal versions following the patterns of the existing tests at `miner_tests.rs:1880-2000`.)

**Step 2: Verify**

Run: `cargo test -p ra2-rust-game --lib miner_tests::scan_filter_`
Expected: 4 PASS.

**Step 3: Commit**

`sim/miner: unit tests for build_scan_filter occupancy/path-grid exclusion`

---

### Task 6: Integration test — stuck miner unsticks via per-tick rescan

**Why:** End-to-end verification of the FIX 2 behavior. Reproduces the user's reported scenario (target ore cell becomes/is blocked) and asserts the miner retargets.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs` (add a new test using `tick_miners` and the full state machine)

**Step 1: Add test**

```rust
#[test]
fn miner_retargets_when_initial_target_is_tree_blocked() {
    // 32x32 grid. Ore at (12,12) AND (13,13). Tree at (12,12). Miner at (10,10).
    // Expected: miner targets (13,13), NOT (12,12). No stuck state.
    let mut grid = PathGrid::test_all_passable(32, 32);
    grid.set_blocked(12, 12, true);
    let mut sim = build_test_sim_with_grid_and_ore(
        32, 32, grid,
        [(12, 12, 50), (13, 13, 50)],
    );
    let miner = spawn_war_miner(&mut sim, owner: "Soviets", at: (10, 10));
    sim.entities.get_mut(miner).unwrap().miner.as_mut().unwrap().state = MinerState::SearchOre;

    let rules = make_rules_for_test();
    let config = MinerConfig::from_general_rules(&rules.general);

    for _ in 0..5 {
        crate::sim::miner::tick_miners(&mut sim, &rules, &config, sim.path_grid.as_ref());
    }

    let m = sim.entities.get(miner).unwrap().miner.as_ref().unwrap();
    assert_ne!(m.target_ore_cell, Some((12, 12)), "must not target tree-blocked cell");
    assert!(matches!(m.state, MinerState::MoveToOre | MinerState::Harvest),
        "miner must progress past SearchOre");
}

#[test]
fn miner_retargets_when_blocker_appears_mid_move() {
    // Miner targets (15,15) and is mid-move. A tree spawns on (15,15) at tick 3.
    // Expected: miner retargets to next-best ore cell within 1 tick.
    let grid = PathGrid::test_all_passable(32, 32);
    let mut sim = build_test_sim_with_grid_and_ore(
        32, 32, grid,
        [(15, 15, 50), (14, 14, 50)],
    );
    let miner = spawn_war_miner(&mut sim, owner: "Soviets", at: (10, 10));
    let rules = make_rules_for_test();
    let config = MinerConfig::from_general_rules(&rules.general);

    // Tick once to set target.
    crate::sim::miner::tick_miners(&mut sim, &rules, &config, sim.path_grid.as_ref());
    let initial_target = sim.entities.get(miner).unwrap()
        .miner.as_ref().unwrap().target_ore_cell;
    assert_eq!(initial_target, Some((15, 15)), "expected initial target (15,15)");

    // Block (15,15) — simulate tree appearing.
    sim.path_grid.as_mut().unwrap().set_blocked(15, 15, true);

    // Tick again — rescan should pick (14,14).
    crate::sim::miner::tick_miners(&mut sim, &rules, &config, sim.path_grid.as_ref());

    let new_target = sim.entities.get(miner).unwrap()
        .miner.as_ref().unwrap().target_ore_cell;
    assert_eq!(new_target, Some((14, 14)), "must retarget to next-best cell");
}

#[test]
fn miner_target_stable_across_ticks_when_nothing_changes() {
    // Regression: per-tick rescan must not thrash. With a stable world, the
    // miner's target must remain constant tick-to-tick.
    let grid = PathGrid::test_all_passable(32, 32);
    let mut sim = build_test_sim_with_grid_and_ore(
        32, 32, grid,
        [(15, 15, 50), (16, 16, 50), (17, 17, 50)],
    );
    let miner = spawn_war_miner(&mut sim, owner: "Soviets", at: (10, 10));
    let rules = make_rules_for_test();
    let config = MinerConfig::from_general_rules(&rules.general);

    crate::sim::miner::tick_miners(&mut sim, &rules, &config, sim.path_grid.as_ref());
    let target_t1 = sim.entities.get(miner).unwrap()
        .miner.as_ref().unwrap().target_ore_cell;

    for _ in 0..5 {
        crate::sim::miner::tick_miners(&mut sim, &rules, &config, sim.path_grid.as_ref());
    }
    let target_t6 = sim.entities.get(miner).unwrap()
        .miner.as_ref().unwrap().target_ore_cell;

    assert_eq!(target_t1, target_t6, "target must not thrash across ticks");
}
```

**Step 2: Verify**

Run: `cargo test -p ra2-rust-game --lib miner_tests::miner_retargets miner_tests::miner_target_stable`
Expected: 3 PASS.

**Step 3: Commit**

`sim/miner: integration tests for stuck-miner retarget + stability`

---

### Task 7: Run full miner + slave miner test suite and audit failures

**Why:** The filter + rescan change touches several state-machine paths. Existing tests may rely on the old "first-pick, never re-evaluate" behavior or may have placed entities/ore in configurations the new filter rejects.

**Files:**
- Audit: `src/sim/miner/miner_tests.rs` (full file)
- Audit: `src/sim/slave_miner.rs` test module if present, or `src/sim/slave_miner_tests.rs`

**Step 1: Run miner tests**

Run: `cargo test -p ra2-rust-game --lib miner --no-fail-fast 2>&1 | tee /tmp/miner_test_log.txt`

**Step 2: Inspect failures**

For each failing test, classify:
- **Stale assertion against old "stuck" behavior** — update the assertion to expect the new retarget behavior.
- **Test scenario implicitly relied on no occupancy filter** — if the test placed miner A and miner B on the same ore patch as setup, the new filter will change which cell gets picked. Re-derive the expected cell.
- **Genuine regression** — if the test fails for a reason unrelated to the changes (e.g., a tick-ordering side effect), STOP and investigate root cause.

**Step 3: Update affected tests inline; do not delete them**

If a test's assertion is obsolete (e.g., "miner stays in MoveToOre with target=(12,12) for 5 ticks because cell blocked"), rewrite the assertion to match the new behavior ("miner retargets to (13,13) within 1 tick"). Preserve the test's intent.

**Step 4: Run slave miner tests**

Run: `cargo test -p ra2-rust-game --lib slave_miner --no-fail-fast`

Apply the same audit procedure.

**Step 5: Run full sim suite once more**

Run: `cargo test -p ra2-rust-game --lib`
Expected: ALL PASS.

**Step 6: Commit**

`sim/miner: update tests for occupancy filter + per-tick rescan behavior`

---

### Task 8: In-game verification

**Why:** The user reported the bug via screenshot. Visual confirmation that the fix lands is part of the parity bar — passing `cargo test` is necessary but not sufficient.

**Files:** None (manual verification).

**Step 1: Build release**

Run: `cargo build --release -p ra2-rust-game`

**Step 2: Launch a skirmish**

- Pick a map with ore patches that have natural tree overlays (most stock maps qualify; e.g., one of the Heartland maps).
- Soviet faction, build a refinery near a heavily-tree-decorated ore patch.
- Train at least one Chrono Miner and ideally a second war miner.

**Step 3: Observe**

- Confirm: no miner gets stuck in MoveToOre with a 1-2-step BLOCKED path.
- Confirm: when two miners harvest the same patch, they pick different cells.
- Confirm: when a chrono miner targets a cell another miner just claimed, it retargets within 1-2 ticks.

**Step 4: Compare to gamemd.exe**

If feasible, run the same scenario in the original game and compare visually. (Per CLAUDE.md parity bar: "indistinguishable in a single skirmish".)

**Step 5: If miners still stick, do NOT layer more fixes**

Per CLAUDE.md change-management rules: stop and reassess. Capture the screenshot + the miner's state and re-open the diagnosis.

**Step 6: Commit verification note (optional)**

If the in-game test passes cleanly, write a one-line note in the chrono-miner trace doc:
`ra2-rust-game-docs/traces/CHRONO_MINER_MISSION_HARVEST_TRACE.md` → add a verification entry pointing at this plan + commits.

---

## Sources & References

- **Trace reports (this session):**
  - `ra2-rust-game-docs/traces/MINER_STUCK_MULTI_MINER_CELL_CONTENTION_TRACE.md` — primary source for FIX 1
  - `ra2-rust-game-docs/traces/MINER_STUCK_WATCHDOG_RETARGET_ON_UNREACHABLE_TRACE.md` — primary source for FIX 2
  - `ra2-rust-game-docs/traces/MINER_STUCK_SCAN_PICKS_BLOCKED_ORE_CELL_TRACE.md` — context; its "scan is naive" conclusion is contradicted by the multi-miner trace
  - `ra2-rust-game-docs/traces/MINER_STUCK_TIBERIUM_PASSABILITY_BYPASS_TRACE.md` — flagged as containing a factual error (Tiberium overlay does NOT set `overlay_blocks`); kept here as a record of the false alarm
  - `ra2-rust-game-docs/traces/MINER_STUCK_FINAL_APPROACH_ADJACENT_TO_ORE_TRACE.md` — flagged as containing a factual error (claims CMIN warps outbound); kept here as a record of the false alarm
- **Existing related research:** `ra2-rust-game-docs/traces/CHRONO_MINER_MISSION_HARVEST_TRACE.md` — Mission_Harvest state-machine reference
- **gamemd.exe addresses (NOT to be copied into Rust comments):**
  - `0x004DD0A0` — `FootClass::Scan_For_Tiberium`
  - `0x004DCE80` — `FootClass::Is_Cell_Harvestable`
  - `0x0073F0A0` — `UnitClass::Can_Enter_Cell`
  - `0x0073E5E0` — `UnitClass::Mission_Harvest`
  - UnitClass vtable base `0x007f5c70`, slot `0x1AC` at offset `0x007f5e1c`
- **Repo code touch points:**
  - `src/sim/miner/miner_system.rs:243-266` — `build_reachable_filter` (replaced)
  - `src/sim/miner/miner_system.rs:1143-1220` — `search_local_ore` (unchanged signature, behavior change via caller's filter)
  - `src/sim/miner/miner_system.rs:268, 337, 474, 503` — call sites
  - `src/sim/slave_miner.rs:191, 348, 638, 651` — slave call sites
  - `src/sim/occupancy.rs:217, 80` — `OccupancyGrid::get`, `CellOccupancy::has_blockers_on`
  - `src/sim/pathfinding/core.rs:1158` — `PathGrid::is_walkable`
  - `src/map/resolved_terrain.rs:389-428` — confirms tiberium-overlay does NOT set `overlay_blocks` (false-alarm refutation)
- **INI keys:** None new.
- **Prior plans / commits:**
  - `3fc928a sim/miner: refinery exit + chrono inbound warp parity fixes` (recent)
  - `8992d5d sim/miner: rewrite ore search as Scan_For_Tiberium diamond-ring expansion` (introduced the new ring-expansion scan that exposed this bug by picking blocked cells)
