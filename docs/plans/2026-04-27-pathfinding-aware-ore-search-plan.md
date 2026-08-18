# Pathfinding-Aware Ore Search — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Filter ore-search candidates by zone-based reachability so harvesters never target unreachable ore (matching gamemd's `Is_Cell_Harvestable` → `Can_Reach_Zone` predicate).

**Architecture:** Two existing search functions (`search_local_ore`, `pick_best_resource_node`) gain an optional filter closure. `handle_search_ore` reads `sim.zone_grid` and `entity.locomotor.movement_zone` directly, builds a closure that probes each ore candidate's 8 neighbors against `ZoneGrid::can_reach`, and passes it down. No new parameters threaded through `tick_miners`/`tick_resource_economy`.

**Design Doc:** [docs/plans/2026-04-27-pathfinding-aware-ore-search-design.md](2026-04-27-pathfinding-aware-ore-search-design.md)

---

## Grounding Summary

- **Docs say:** gamemd's `FootClass::Scan_For_Tiberium` calls `Is_Cell_Harvestable` per candidate, which gates on `Can_Reach_Zone(unit_zone, target_cell)`. Ore search is reachability-filtered at scan time, not at move time. Confirmed via [MISSION_HARVEST_GHIDRA_REPORT.md §4.1](../../../ra2-rust-game-docs/MISSION_HARVEST_GHIDRA_REPORT.md), [HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md), and [FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md §4–5](../../../ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md).
- **Ghidra confirmed (this session):** `FootClass::Scan_For_Tiberium @ 0x4DD0A0` is a diamond-spiral scan with per-candidate predicate. `FootClass::Is_Cell_Harvestable @ 0x4DCE80` calls `MapClass::Can_Reach_Zone(unit_zone, target)` plus playfield/shroud/LandType/Can_Enter_Cell checks.
- **Repo pattern this mirrors:** closure-as-predicate is already used in [pathfinding::core search options](../../../src/sim/pathfinding/core.rs). 8-neighbor probe pattern matches [overlay_grid.rs ADJACENT_8](../../../src/sim/overlay_grid.rs#L334), [bump_crush.rs NEIGHBOR_OFFSETS](../../../src/sim/movement/bump_crush.rs#L74), and [ore_growth.rs ADJACENT_OFFSETS](../../../src/sim/ore_growth.rs#L42).
- **INI keys driving behavior:** none new. Existing `TiberiumShortScan`, `TiberiumLongScan` (already INI-driven via `MinerConfig::from_general_rules`) define scan radii; this change does not touch them.
- **Still unknown:** none. The design is fully grounded.

## Key Technical Decisions

- **Filter is a closure, not a concrete `&ZoneGrid` parameter** — keeps `search_local_ore`/`pick_best_resource_node` reachability-agnostic and matches gamemd's per-candidate-predicate structure. — **Confidence:** high — **Source:** design doc Approach 1 (chosen during brainstorm)
- **Reachability check uses ore cell's 8 neighbors, not the ore cell itself** — Tiberium cells are marked impassable in the path grid (so A* doesn't path through ore fields), so `zone_at(ore_cell)` is always `ZONE_INVALID`. Equivalent gamemd behavior is achieved by checking whether any passable neighbor of the ore is in the harvester's zone. — **Confidence:** high — **Source:** verified path-grid construction in [src/sim/pathfinding/zone_build.rs is_passable](../../../src/sim/pathfinding/zone_build.rs#L440), repo grep confirms Tiberium → PASS_BLOCKED at [passability.rs §matrix table](../../../src/sim/pathfinding/passability.rs#L115)
- **Effective-zone-cell probe (self + 8 neighbors)** for the harvester when standing on Tiberium itself — answer is unique up to neighbor-iteration order (deterministic). — **Confidence:** high — **Source:** Q2=A in brainstorm; mirrors gamemd's reliance on the harvester being on a passable cell at search time (gamemd always emerges from harvest before scanning; Rust's tick ordering can scan mid-Tiberium-cell so we need the probe).
- **Skip the slave miner** — slave miner has its own search system in [src/sim/slave_miner.rs](../../../src/sim/slave_miner.rs). Wiring zone-aware filtering there is a separate concern; existing slave_miner callers of `search_local_ore` pass `None` and observe unchanged behavior. — **Confidence:** high — **Source:** scope decision in design doc Impact Analysis

## Open Questions

### Resolved During Planning

- "Do we need to thread `&ZoneGrid` through `tick_miners`/`tick_resource_economy`?" — **No**, `handle_search_ore` takes `sim: &Simulation` and can read `sim.zone_grid.as_ref()` directly. Verified at [miner_system.rs:209](../../../src/sim/miner/miner_system.rs#L209).
- "Where does the harvester's `MovementZone` come from?" — `entity.locomotor.as_ref()?.movement_zone`. Confirmed at [game_entity.rs:87](../../../src/sim/game_entity.rs#L87) and [movement/locomotor.rs:170](../../../src/sim/movement/locomotor.rs#L170). Default to `MovementZone::Normal` if locomotor missing (test setups).
- "How do we get the movement layer (ground vs bridge)?" — `entity.movement_layer_or_ground()` exists at [game_entity.rs:317](../../../src/sim/game_entity.rs#L317).
- "Will the closure live long enough across the three search rings?" — yes; we build a single boxed closure or a stack-bound closure once per `handle_search_ore` call and pass `Some(&closure)` into each ring. Closure captures `&ZoneGrid` and a single `(u16, u16)` reference cell, both Copy/&-bound.
- "Does the existing `pick_best_resource_node` test break if we add a filter parameter?" — yes mechanically; the two tests at [miner_tests.rs:955, 989](../../../src/sim/miner/miner_tests.rs#L955) call it with the old signature. Pass `None` for the new parameter — observable behavior unchanged.

### Deferred to Implementation

- None. All decisions are settled.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/miner/miner_system.rs](../../../src/sim/miner/miner_system.rs) | Add filter parameter to `search_local_ore`; build & pass filter from `handle_search_ore`; add `effective_zone_cell` and `ore_reachable` helpers |
| Modify | [src/sim/production/production_economy.rs](../../../src/sim/production/production_economy.rs) | Add filter parameter to `pick_best_resource_node` |
| Modify | [src/sim/slave_miner.rs](../../../src/sim/slave_miner.rs) | Pass `None` for the new filter at four existing `search_local_ore` call sites |
| Modify | [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs) | Pass `None` for the new filter at two existing `pick_best_resource_node` call sites; add three new reachability tests |

## Interface Changes

- `search_local_ore` (pub(crate), re-exported) — gains last parameter `filter: Option<&dyn Fn((u16, u16)) -> bool>`. Existing callers must pass `None`. Slave miner (3 call sites) and tests are updated.
- `pick_best_resource_node` (pub) — gains last parameter `filter: Option<&dyn Fn((u16, u16)) -> bool>`. Existing callers must pass `None`. Two test call sites are updated.

No new pub fns/structs/enums. No INI keys.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 in game logic (no math added; only zone-id comparisons and 8-neighbor coordinate arithmetic on `u16`/`i32`)
- [x] New state included in deterministic state hash — no new state added
- [x] No dependencies on render/ui/sidebar/audio/net — only `sim/miner` ↔ `sim/production` ↔ `sim/pathfinding`
- [x] Tick ordering impact noted — none; `handle_search_ore` runs in the existing miner state-machine slot; `zone_grid` is read-only
- [x] BTreeMap iteration order considered — `search_local_ore` iterates `BTreeMap<(u16, u16), ResourceNode>` in deterministic key order; the filter doesn't change iteration order, only acceptance

## Risk Areas

- **Existing tests that pass empty `PathGrid::new(64, 64)` and no `ZoneGrid`** — when `sim.zone_grid` is `None`, the closure builder returns `None`; search runs unfiltered (current behavior). Verified by re-reading [tick_miners snapshot path](../../../src/sim/miner/miner_system.rs#L51): no test that doesn't construct a `ZoneGrid` should fail.
- **Slave miner regression** — four existing `search_local_ore` call sites need `, None)` added (slave_miner.rs:190, 333, 615, 621). Mechanical; covered by `cargo check`.
- **Filter performance** — short scan = ~113 cells × 8 neighbors = ~904 zone lookups per search. Long scan = ~7200 × 8 = ~57600 lookups. Each is O(1) array index in `ZoneMap::zone_at` plus an O(1) super-zone union-find query. At 15 Hz with ≤ 30 harvesters per player and search infrequent (only on cell-depleted or post-undock), well below the per-tick budget. No new allocations in the hot path (closure captures by reference).
- **Determinism: 8-neighbor probe order** — fixed via the `ADJACENT_8` constant; identical iteration order on every machine.

### Known limitation (carried from disparity scan)

- Rust's `search_local_ore` scans the radius as a circular bounded box and picks "best across the radius"; gamemd's `Scan_For_Tiberium` is a diamond-spiral with ring-by-ring early-exit. Picked cells can differ when ore density varies sharply across the radius. Flagged as LOW-severity in [docs/gap-scans/2026-04-27-disparity-scan-miner.md](../gap-scans/2026-04-27-disparity-scan-miner.md). Out of scope for this plan; a separate brainstorm if it ever surfaces as observable.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | Reachability filter wiring | Player-observable: chrono miner with refinery placed adjacent to back-side ore no longer drives into the wall | Manual in-game observation in Task 8 + unit test in Task 5 |
| Task 4 | Effective-zone-cell probe (mid-Tiberium harvester) | Harvester mid-cell-depletion search must still filter; otherwise the bug recurs the moment a harvester re-scans while standing on ore | Unit test in Task 7 |

---

## Tasks

### Task 1: Add filter parameter to `search_local_ore`

**Why:** This is the core search primitive; updating it first unblocks all callers. Order-1 because it's the lowest-level interface change.

**Files:**
- Modify: [src/sim/miner/miner_system.rs:863-901](../../../src/sim/miner/miner_system.rs#L863)

**Pattern:** Optional filter closure parameter; matches `Option<&dyn Fn(...)>` style used elsewhere in the codebase.

**Step 1: Update signature and apply filter inside the loop**

Replace the function at [miner_system.rs:867-901](../../../src/sim/miner/miner_system.rs#L867) with:

```rust
pub(crate) fn search_local_ore(
    nodes: &std::collections::BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
) -> Option<(u16, u16)> {
    let mut best: Option<((u8, u32, u32, u16, u16), (u16, u16))> = None;
    let min_x = center.0.saturating_sub(radius);
    let max_x = center.0.saturating_add(radius);
    let min_y = center.1.saturating_sub(radius);
    let max_y = center.1.saturating_add(radius);

    for (&(rx, ry), node) in nodes {
        if node.remaining == 0 || rx < min_x || rx > max_x || ry < min_y || ry > max_y {
            continue;
        }
        let dx = rx as i64 - center.0 as i64;
        let dy = ry as i64 - center.1 as i64;
        let dist_sq = (dx * dx + dy * dy) as u32;
        if dist_sq > (radius as u32) * (radius as u32) {
            continue; // circular, not square
        }
        if let Some(f) = filter {
            if !f((rx, ry)) {
                continue;
            }
        }
        let type_rank: u8 = if node.resource_type == ResourceType::Ore {
            1
        } else {
            0
        };
        let density_rank: u32 = u32::MAX - node.remaining as u32;
        let rank = (type_rank, density_rank, dist_sq, ry, rx);
        match best {
            Some((ref cur, _)) if rank >= *cur => {}
            _ => best = Some((rank, (rx, ry))),
        }
    }
    best.map(|(_, cell)| cell)
}
```

**Step 2: Verify the file still compiles standalone**

Run: `cargo check -p vera20k --lib 2>&1 | grep -E "error\[E|^error"`

Expected: errors at the existing call sites (miner_system.rs lines 218 and 249, slave_miner.rs three sites, miner_tests.rs tests) due to missing 4th argument. This is expected — fixed in Tasks 2 and 3.

**Step 3: Do NOT commit yet.** Subsequent tasks fix the call sites; we want a green commit, not a broken one.

### Task 2: Add filter parameter to `pick_best_resource_node`

**Why:** Companion change to Task 1 for the global-fallback path. Same shape, separate file.

**Files:**
- Modify: [src/sim/production/production_economy.rs:33-65](../../../src/sim/production/production_economy.rs#L33)

**Pattern:** Same as Task 1.

**Step 1: Update signature and apply filter**

Replace the function at [production_economy.rs:34-65](../../../src/sim/production/production_economy.rs#L34) with:

```rust
/// Find the nearest non-empty resource node to `from`.
///
/// `filter`, if provided, is called per candidate; only cells for which
/// it returns `true` are considered. Pass `None` for unfiltered behavior.
pub fn pick_best_resource_node(
    nodes: &BTreeMap<(u16, u16), ResourceNode>,
    from: (u16, u16),
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
) -> Option<(u16, u16)> {
    // RA2 cell selection priority (ref doc §3):
    //   1. Gems over ore (type_rank: 0=gem, 1=ore)
    //   2. Highest density (density_rank: inverted remaining so more = lower = better)
    //   3. Nearest (dist_sq)
    //   4. Deterministic tie-break (ry, rx)
    let mut best: Option<((u8, u32, u32, u16, u16), (u16, u16))> = None;
    for (&(rx, ry), node) in nodes {
        if node.remaining == 0 {
            continue;
        }
        if let Some(f) = filter {
            if !f((rx, ry)) {
                continue;
            }
        }
        let dx = rx as i64 - from.0 as i64;
        let dy = ry as i64 - from.1 as i64;
        let dist_sq = (dx * dx + dy * dy) as u32;
        let type_rank: u8 = if node.resource_type == ResourceType::Ore {
            1
        } else {
            0
        };
        // Invert remaining so higher density = lower rank = preferred.
        let density_rank: u32 = u32::MAX - node.remaining as u32;
        let rank = (type_rank, density_rank, dist_sq, ry, rx);
        match best {
            Some((ref cur, _)) if rank >= *cur => {}
            _ => best = Some((rank, (rx, ry))),
        }
    }
    best.map(|(_, cell)| cell)
}
```

**Step 2: Do NOT commit yet.** Same reason as Task 1.

### Task 3: Update existing call sites to pass `None`

**Why:** Restore green compile at the call sites that aren't doing reachability filtering — slave miner and tests. After this task, every `search_local_ore`/`pick_best_resource_node` callsite compiles; only Task 4 actually wires up reachability filtering.

**Files:**
- Modify: [src/sim/slave_miner.rs:190, 333, 615, 621](../../../src/sim/slave_miner.rs#L190)
- Modify: [src/sim/miner/miner_tests.rs:955, 989](../../../src/sim/miner/miner_tests.rs#L955)
- Modify: [src/sim/miner/miner_system.rs:218, 249, 260](../../../src/sim/miner/miner_system.rs#L218) — temporarily pass `None`; Task 4 replaces these with the real filter.

**Pattern:** Add `, None` as the new last argument.

**Step 1: Update slave_miner.rs:190**

In [src/sim/slave_miner.rs:190](../../../src/sim/slave_miner.rs#L190), replace:

```rust
    if let Some(cell) = search_local_ore(&sim.production.resource_nodes, master_pos, scan_radius) {
```

with:

```rust
    if let Some(cell) =
        search_local_ore(&sim.production.resource_nodes, master_pos, scan_radius, None)
    {
```

**Step 2: Update slave_miner.rs:333**

Same pattern at [slave_miner.rs:333](../../../src/sim/slave_miner.rs#L333). Replace:

```rust
    if let Some(cell) = search_local_ore(&sim.production.resource_nodes, master_pos, scan_radius) {
```

with:

```rust
    if let Some(cell) =
        search_local_ore(&sim.production.resource_nodes, master_pos, scan_radius, None)
    {
```

**Step 3: Update slave_miner.rs:615 and slave_miner.rs:621**

These two calls live back-to-back in `check_scan_correction` (the slave-miner reposition heuristic). Update both.

At [slave_miner.rs:615](../../../src/sim/slave_miner.rs#L615), replace:

```rust
    let current_nearest = search_local_ore(&sim.production.resource_nodes, (mrx, mry), short_scan)?;
```

with:

```rust
    let current_nearest =
        search_local_ore(&sim.production.resource_nodes, (mrx, mry), short_scan, None)?;
```

At [slave_miner.rs:621](../../../src/sim/slave_miner.rs#L621) (a few lines below, in the same function), replace:

```rust
    let better_ore = search_local_ore(&sim.production.resource_nodes, (mrx, mry), long_scan)?;
```

with:

```rust
    let better_ore =
        search_local_ore(&sim.production.resource_nodes, (mrx, mry), long_scan, None)?;
```

**Step 4: Update miner_tests.rs:955**

In [src/sim/miner/miner_tests.rs:955](../../../src/sim/miner/miner_tests.rs#L955), replace:

```rust
    let chosen = pick_best_resource_node(&nodes, (5, 5));
```

with:

```rust
    let chosen = pick_best_resource_node(&nodes, (5, 5), None);
```

**Step 5: Update miner_tests.rs:989**

Same pattern at [miner_tests.rs:989](../../../src/sim/miner/miner_tests.rs#L989). Replace:

```rust
    let chosen = pick_best_resource_node(&nodes, (5, 5));
```

with:

```rust
    let chosen = pick_best_resource_node(&nodes, (5, 5), None);
```

**Step 6: Temporarily update miner_system.rs callsites in `handle_search_ore`**

At [miner_system.rs:218](../../../src/sim/miner/miner_system.rs#L218), replace the call:

```rust
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        search_center,
        config.local_continuation_radius,
    ) {
```

with:

```rust
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        search_center,
        config.local_continuation_radius,
        None,
    ) {
```

At [miner_system.rs:249](../../../src/sim/miner/miner_system.rs#L249), replace:

```rust
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.long_scan_radius,
    ) {
```

with:

```rust
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.long_scan_radius,
        None,
    ) {
```

At [miner_system.rs:260](../../../src/sim/miner/miner_system.rs#L260), replace:

```rust
    if let Some(cell) = pick_best_resource_node(&sim.production.resource_nodes, (snap.rx, snap.ry))
    {
```

with:

```rust
    if let Some(cell) =
        pick_best_resource_node(&sim.production.resource_nodes, (snap.rx, snap.ry), None)
    {
```

These three callsites are placeholders for Task 4 (which replaces `None` with the actual reachability filter). Keeping them `None` here ensures Tasks 1–3 form a green-compile checkpoint before the harder change.

**Step 7: Verify compile**

Run: `cargo check -p vera20k --lib`

Expected: clean compile (warnings allowed; errors not).

**Step 8: Run existing tests**

Run: `cargo test -p vera20k --lib miner`

Expected: PASS for all 51 existing miner tests.

**Step 9: Commit**

```
git add src/sim/miner/miner_system.rs src/sim/production/production_economy.rs \
        src/sim/slave_miner.rs src/sim/miner/miner_tests.rs
git commit -m "miner: add filter param to search_local_ore + pick_best_resource_node"
```

### Task 4: Wire up the reachability filter in `handle_search_ore`

**Why:** Production change. Builds the closure from `sim.zone_grid` + harvester effective zone cell; passes it through all three rings.

**Files:**
- Modify: [src/sim/miner/miner_system.rs:209-270](../../../src/sim/miner/miner_system.rs#L209) — `handle_search_ore`; replace the three `None` filter args from Task 3
- Modify: [src/sim/miner/miner_system.rs](../../../src/sim/miner/miner_system.rs) — add two private helpers (`ADJACENT_8`, `effective_zone_cell`, `ore_reachable`)

**Pattern:** Existing handler reads `sim` and `snap`; we add data lookups at the top. Closure captures by reference; no allocations in the hot path.

**Step 1: Add the imports and 8-neighbor constant near the top of the file**

Find the imports block at the top of [miner_system.rs](../../../src/sim/miner/miner_system.rs). Add the following imports if they are not already present (verify by reading the existing import block — do not duplicate):

```rust
use crate::rules::locomotor_type::MovementZone;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::zone_map::{ZONE_INVALID, ZoneGrid};
```

Below the imports, near other module-level constants (search the file for existing `const` declarations to find a sensible location), add:

```rust
/// 8-neighbor offsets in clockwise order starting from north. Used by the
/// effective-zone-cell probe and the ore-reachability check.
const ADJACENT_8: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];
```

**Step 2: Add the `effective_zone_cell` helper**

Place this private function next to `search_local_ore` (above or below — whatever sits cleanly; suggest immediately above `search_local_ore` near [line 867](../../../src/sim/miner/miner_system.rs#L867)):

```rust
/// Return a cell whose zone serves as the harvester's reachability anchor.
///
/// The harvester's own cell may be on Tiberium (impassable in the path grid,
/// hence `ZONE_INVALID`); when so, probe its 8 neighbors and return the
/// first cell with a valid zone. Returns `None` if neither the harvester's
/// cell nor any neighbor has a valid zone — caller falls back to no-filter
/// behavior for that tick.
fn effective_zone_cell(
    zone_grid: &ZoneGrid,
    mz: MovementZone,
    rx: u16,
    ry: u16,
) -> Option<(u16, u16)> {
    let zone_map = zone_grid.map_for(mz)?;
    if zone_map.zone_at(rx, ry, MovementLayer::Ground) != ZONE_INVALID {
        return Some((rx, ry));
    }
    for &(dx, dy) in &ADJACENT_8 {
        let nx = (rx as i32).checked_add(dx)?;
        let ny = (ry as i32).checked_add(dy)?;
        if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
            continue;
        }
        let (nx, ny) = (nx as u16, ny as u16);
        if zone_map.zone_at(nx, ny, MovementLayer::Ground) != ZONE_INVALID {
            return Some((nx, ny));
        }
    }
    None
}
```

**Step 3: Add the `ore_reachable` helper**

Place this private function immediately after `effective_zone_cell`:

```rust
/// True if any 8-neighbor of `ore_cell` is in the harvester's connected zone
/// component. Ore cells themselves are `ZONE_INVALID` because Tiberium is
/// blocked in the path grid (so A* doesn't path through ore fields), so we
/// probe the ore's neighbors instead — mirroring how a harvester actually
/// approaches an ore patch.
fn ore_reachable(
    zone_grid: &ZoneGrid,
    mz: MovementZone,
    layer: MovementLayer,
    harvester_zone_cell: (u16, u16),
    ore_cell: (u16, u16),
) -> bool {
    for &(dx, dy) in &ADJACENT_8 {
        let nx = (ore_cell.0 as i32).checked_add(dx);
        let ny = (ore_cell.1 as i32).checked_add(dy);
        if let (Some(nx), Some(ny)) = (nx, ny) {
            if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
                continue;
            }
            let (nx, ny) = (nx as u16, ny as u16);
            if zone_grid.can_reach(mz, harvester_zone_cell, layer, (nx, ny), layer) {
                return true;
            }
        }
    }
    false
}
```

**Step 4: Update `handle_search_ore` to build and pass the filter**

Replace the entire body of [handle_search_ore at miner_system.rs:209-270](../../../src/sim/miner/miner_system.rs#L209) with:

```rust
fn handle_search_ore(
    sim: &Simulation,
    config: &MinerConfig,
    _path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let search_center = snap.miner.last_harvest_cell.unwrap_or((snap.rx, snap.ry));

    // Build a reachability filter from the zone grid + harvester locomotor.
    // If any of (zone_grid, locomotor, effective zone cell) is missing, fall
    // back to unfiltered search for this tick — the next tick will likely
    // succeed once the harvester moves to a passable cell.
    let entity = sim.entities.get(snap.entity_id);
    let mz = entity
        .and_then(|e| e.locomotor.as_ref())
        .map(|loc| loc.movement_zone)
        .unwrap_or(MovementZone::Normal);
    let layer = entity
        .map(|e| e.movement_layer_or_ground())
        .unwrap_or(MovementLayer::Ground);
    let harvester_anchor = sim
        .zone_grid
        .as_ref()
        .and_then(|zg| effective_zone_cell(zg, mz, snap.rx, snap.ry));

    let reachable_filter: Option<Box<dyn Fn((u16, u16)) -> bool + '_>> =
        match (sim.zone_grid.as_ref(), harvester_anchor) {
            (Some(zg), Some(anchor)) => Some(Box::new(move |ore_cell: (u16, u16)| {
                ore_reachable(zg, mz, layer, anchor, ore_cell)
            })),
            _ => None,
        };
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> =
        reachable_filter.as_deref().map(|f| f as &dyn Fn(_) -> _);

    // Try local continuation scan first (short radius around last harvest spot).
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        search_center,
        config.local_continuation_radius,
        filter_ref,
    ) {
        snap.miner.target_ore_cell = Some(cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // gamemd.exe (0x0073E844): both war miners and chrono miners use
    // TiberiumLongScan for the initial search — no early exit for chrono.
    // The only chrono-specific behavior is stopping piggybacked locomotion
    // before the scan, which we handle elsewhere.

    // ArchiveTarget pattern (from RA1): if we remember a productive patch and it
    // still has ore AND it's reachable, go back there before doing a full global search.
    if let Some(archive) = snap.miner.last_harvest_cell {
        let archive_has_ore = sim.production.resource_nodes.contains_key(&archive);
        let archive_reachable = filter_ref.is_none_or(|f| f(archive));
        if archive_has_ore && archive_reachable {
            snap.miner.target_ore_cell = Some(archive);
            snap.miner.state = MinerState::MoveToOre;
            // Clear archive so we don't loop back forever if it depletes on arrival.
            snap.miner.last_harvest_cell = None;
            return;
        }
    }

    // Long-range bounded scan from the miner's current position (TiberiumLongScan).
    if let Some(cell) = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.long_scan_radius,
        filter_ref,
    ) {
        snap.miner.target_ore_cell = Some(cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // Global search — find nearest reachable ore anywhere on the map.
    if let Some(cell) =
        pick_best_resource_node(&sim.production.resource_nodes, (snap.rx, snap.ry), filter_ref)
    {
        snap.miner.target_ore_cell = Some(cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // No reachable ore anywhere.
    snap.miner.state = MinerState::WaitNoOre;
    snap.miner.rescan_cooldown = config.rescan_cooldown_ticks;
}
```

Notes on the changes vs the previous body:
- New: imports/helpers/closure construction at the top.
- New: archive-cell reachability check — old code only required `archive_has_ore`; now also requires `archive_reachable` so a walled-off archive isn't returned to.
- Same: three search rings + global fallback + WaitNoOre.
- The boxing pattern `Box<dyn Fn ... + '_>` is necessary because the closure type contains the captured `&ZoneGrid` lifetime; `Option<Box<...>>` lets us return `None` cleanly. `as_deref` + final cast produces the `Option<&dyn Fn>` shape that `search_local_ore` expects.

**Step 5: Verify compile**

Run: `cargo check -p vera20k --lib`

Expected: clean compile.

**Step 6: Run existing tests**

Run: `cargo test -p vera20k --lib miner`

Expected: all 51 existing miner tests still pass. Tests don't construct a `ZoneGrid`, so `sim.zone_grid` is `None`, so `filter_ref` is `None`, so behavior is identical to pre-change.

**Step 7: Commit**

```
git add src/sim/miner/miner_system.rs
git commit -m "miner: filter ore-search candidates by zone-based reachability

Mirrors gamemd's Is_Cell_Harvestable -> Can_Reach_Zone predicate.
search_local_ore and pick_best_resource_node now accept an optional
filter closure; handle_search_ore builds one from sim.zone_grid that
checks each ore candidate's 8 neighbors against the harvester's
connected zone component. Probes self+8 for the harvester anchor so
the filter still works when the harvester is mid-Tiberium-cell.

Closes the chrono miner head-butt cycle when ore sits behind the
refinery footprint: the back-side ore is in a different zone (or
has no passable neighbor in the harvester's zone) so it is filtered
out, and the search picks reachable front-side ore instead.

Design doc: docs/plans/2026-04-27-pathfinding-aware-ore-search-design.md"
```

### Task 5: Add unit test — `unreachable_ore_filtered_out`

**Why:** Direct unit-level coverage of the core fix. Verifies that ore in a disconnected zone is rejected and the harvester transitions to `WaitNoOre` if no reachable ore exists.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs) — append at end

**Pattern:** Mirrors the existing miner-tests structure (rules + spawn + tick + assert). Adds a `ZoneGrid` build step that the existing tests don't have.

**Step 1: Verify which `ZoneGrid::build` API to use**

Read [src/sim/pathfinding/zone_map.rs:191-258](../../../src/sim/pathfinding/zone_map.rs#L191) and confirm the `ZoneGrid::build(path_grid, terrain_costs, width, height)` signature. The simpler form (no terrain) is used by tests at [zone_map_tests.rs:327](../../../src/sim/pathfinding/zone_map_tests.rs#L327): `ZoneGrid::build(&grid, &BTreeMap::new(), 5, 2)`.

**Step 2: Add the test function**

Append to [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs) (end of file, after the last `#[test]`):

```rust
/// Ore in a disconnected zone (cut off by impassable terrain) must be
/// filtered out by the reachability check. With no reachable ore on the
/// map, the harvester transitions to WaitNoOre rather than picking the
/// unreachable cell.
#[test]
fn unreachable_ore_filtered_out() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // Build a 16x16 path grid with an impassable wall column at x=8 that
    // splits the map into two zones (left and right halves).
    let mut grid = PathGrid::new(16, 16);
    for y in 0..16u16 {
        grid.set_blocked(8, y, true);
    }
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 16, 16);
    sim.zone_grid = Some(zone_grid);

    // Harvester on the LEFT side at (3, 8). Ore on the RIGHT side at (12, 8).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 3, 8);
    place_ore(&mut sim, 12, 8, 1200);

    // Drive the miner into SearchOre state.
    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
    }

    // Tick once — search runs, finds nothing reachable, transitions to WaitNoOre.
    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(
        m.state,
        MinerState::WaitNoOre,
        "must wait — only ore on the map is in a disconnected zone, so unreachable",
    );
    assert!(
        m.target_ore_cell.is_none(),
        "must not have targeted unreachable ore, got {:?}",
        m.target_ore_cell,
    );
}
```

**Step 3: Run the test**

Run: `cargo test -p vera20k --lib unreachable_ore_filtered_out -- --nocapture`

Expected: PASS.

**Step 4: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add unreachable_ore_filtered_out"
```

### Task 6: Add unit test — `reachable_ore_picked_over_closer_unreachable`

**Why:** Verifies the filter doesn't just reject — it also lets the search escalate to a reachable cell that may be farther away.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs) — append at end

**Pattern:** Same as Task 5.

**Step 1: Add the test function**

Append to [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs):

```rust
/// When a closer ore cell is unreachable (different zone) but a farther
/// one is reachable, the harvester must pick the farther reachable cell
/// rather than fall through to WaitNoOre.
#[test]
fn reachable_ore_picked_over_closer_unreachable() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // 16x16 grid with an impassable wall column at x=8.
    let mut grid = PathGrid::new(16, 16);
    for y in 0..16u16 {
        grid.set_blocked(8, y, true);
    }
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 16, 16);
    sim.zone_grid = Some(zone_grid);

    // Harvester at (3, 8) on the LEFT side.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 3, 8);
    // Closer ore at (10, 8) is on the RIGHT side (unreachable).
    place_ore(&mut sim, 10, 8, 1200);
    // Farther ore at (1, 1) is on the LEFT side (reachable).
    place_ore(&mut sim, 1, 1, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.state, MinerState::MoveToOre);
    assert_eq!(
        m.target_ore_cell,
        Some((1, 1)),
        "reachable farther ore at (1,1) must be picked over unreachable closer ore at (10,8). \
         Got {:?}",
        m.target_ore_cell,
    );
}
```

**Step 2: Run the test**

Run: `cargo test -p vera20k --lib reachable_ore_picked_over_closer_unreachable -- --nocapture`

Expected: PASS.

**Step 3: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add reachable_ore_picked_over_closer_unreachable"
```

### Task 7: Add unit test — `harvester_on_tiberium_falls_back_to_neighbor_zone`

**Why:** Covers the mid-harvest re-search case (Q2=A in the brainstorm). The harvester is standing on Tiberium (so `zone_at(harvester)` is `ZONE_INVALID`), and the probe must find an effective zone via a neighbor before the filter applies.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs) — append at end

**Pattern:** Same as Tasks 5–6. Adds an ore overlay at the harvester's position to simulate the standing-on-Tiberium case.

**Step 1: Verify the path-grid blocking model**

Read [src/sim/pathfinding/zone_build.rs is_passable at line 440](../../../src/sim/pathfinding/zone_build.rs#L440). The path grid only marks Tiberium as impassable when terrain data is provided; tests using a bare `PathGrid::new` and `BTreeMap::new()` for terrain costs will have all unblocked cells in the same zone. To force an in-test "harvester on impassable cell" scenario without resolving terrain, manually `set_blocked(rx, ry, true)` for the harvester's cell.

**Step 2: Add the test function**

Append to [src/sim/miner/miner_tests.rs](../../../src/sim/miner/miner_tests.rs):

```rust
/// When the harvester is standing on a cell marked impassable in the path
/// grid (mirrors mid-harvest on Tiberium), the effective-zone probe must
/// find a valid zone via a neighbor and the filter must still apply.
/// Specifically: nearby reachable ore is picked, distant unreachable ore
/// is filtered.
#[test]
fn harvester_on_tiberium_falls_back_to_neighbor_zone() {
    use crate::sim::pathfinding::zone_map::ZoneGrid;
    use std::collections::BTreeMap;

    let mut sim = Simulation::new();
    let rules = miner_rules();

    // 16x16 grid. Wall column at x=8 splits LEFT and RIGHT zones.
    // Harvester's cell at (3, 8) is also blocked (simulates standing on
    // Tiberium that the path grid marks impassable).
    let mut grid = PathGrid::new(16, 16);
    for y in 0..16u16 {
        grid.set_blocked(8, y, true);
    }
    grid.set_blocked(3, 8, true);
    let zone_grid = ZoneGrid::build(&grid, &BTreeMap::new(), 16, 16);
    sim.zone_grid = Some(zone_grid);

    // Harvester at (3, 8) on the blocked cell.
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 3, 8);
    // Reachable ore at (5, 8) on the LEFT side.
    place_ore(&mut sim, 5, 8, 1200);
    // Unreachable ore at (10, 8) on the RIGHT side.
    place_ore(&mut sim, 10, 8, 1200);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::SearchOre;
    }

    tick_miners_n(&mut sim, &rules, 1);

    let m = get_miner(&sim, miner_id);
    assert_eq!(m.state, MinerState::MoveToOre);
    assert_eq!(
        m.target_ore_cell,
        Some((5, 8)),
        "left-side reachable ore must be picked even with the harvester on a \
         blocked cell — the effective-zone probe finds a passable neighbor. \
         Got {:?}",
        m.target_ore_cell,
    );
}
```

**Step 3: Run the test**

Run: `cargo test -p vera20k --lib harvester_on_tiberium_falls_back_to_neighbor_zone -- --nocapture`

Expected: PASS.

**Step 4: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add harvester_on_tiberium_falls_back_to_neighbor_zone"
```

### Task 8: Run full test suite + clippy

**Why:** Catch any cross-test interaction or downstream breakage before manual verification.

**Files:** none modified.

**Step 1: Run all lib tests**

Run: `cargo test -p vera20k --lib`

Expected: PASS, 0 failures, 0 ignored newly. The new tests bring the total miner tests to 54+; full suite should remain green.

**Step 2: Run clippy on the changed files**

Run: `cargo clippy -p vera20k --lib 2>&1 | grep -B1 -A6 -E "miner_system|production_economy|slave_miner|miner_tests" | head -80`

Expected: any clippy output for the changed files reflects pre-existing warnings only (the codebase has many pre-existing clippy warnings outside the miner system). No new warnings introduced by the changes.

If clippy reports a NEW warning in any of the modified files (i.e., a warning at a line we just wrote, not at pre-existing lines), fix it before continuing.

**Step 3: If anything fails**

Investigate the failure. If it's a downstream interaction not anticipated by the design doc's Risk Areas section, stop and reassess — do not patch tests reflexively. The fix may have surfaced a real second-order issue.

### Task 9: Manual in-game verification

**Why:** Per CLAUDE.md "verify the end-to-end result of every change, not just the mechanical task." Unit tests verify the filter behaves correctly in isolation; only running the game verifies the head-butt bug is actually closed.

**Files:** none modified.

**Step 1: Build and run the game**

Run: `cargo run -p vera20k --release`

**Step 2: Reproduce the original bug scenario**

Set up a skirmish on a small map where:
- A refinery is placed adjacent to an ore patch on its back side (away from the dock pad's exit cell).
- The chrono miner harvests, returns, and dumps.

This is the same scenario that exhibited the head-butt cycle prior to this fix.

**Step 3: Observe the post-undock cycle**

Wait for the chrono miner to complete a full cycle:
1. Teleport to ore.
2. Harvest.
3. Return / teleport to refinery.
4. Dock and unload.
5. Exit the dock pad.
6. Re-search for ore.

**Expected:** After step 5, the chrono miner picks an ore cell **on the front side of the refinery** (not the back side). No head-butting against the refinery wall. The miner harvests the next load without visible oscillation.

**Step 4: If the symptom persists**

Stop and re-investigate. Possible causes:
- The harvester's `MovementZone` differs from what I assumed (default `Normal`); check the `entity.locomotor.movement_zone` value at runtime via debug output.
- The path grid's Tiberium-blocking is conditional on terrain data that isn't built in this map; if so, ore cells may NOT be `ZONE_INVALID` in the map's zone_grid, which would make the 8-neighbor probe in `ore_reachable` return false on a same-zone ore cell. (Easy to verify with a quick log line in `handle_search_ore`.)
- The `zone_grid` itself is `None` at runtime for this map; the filter degrades to no-op. Check `sim.zone_grid.is_some()` at the call site.

Do not patch over the symptom — diagnose first.

**Step 5: If verified fixed**

Add a one-liner to the design doc's "Definition of Done" section confirming the manual verification was done (date + brief outcome). This is the only doc edit done outside the implementation tasks.

---

## Sources & References

- **Design doc:** [docs/plans/2026-04-27-pathfinding-aware-ore-search-design.md](2026-04-27-pathfinding-aware-ore-search-design.md)
- **Disparity scan:** [docs/gap-scans/2026-04-27-disparity-scan-miner.md](../gap-scans/2026-04-27-disparity-scan-miner.md) — established the gap and prioritized this fix as G1 HIGH severity.
- **Ghidra reports:**
  - [MISSION_HARVEST_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/MISSION_HARVEST_GHIDRA_REPORT.md) §4.1 — Is_Cell_Harvestable + Can_Reach_Zone predicate
  - [HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md) §2 — Mission_Harvest state machine
  - [FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md) — Can_Reach_Zone usage
  - [FOOTCLASS_PATHFINDING_AND_MOVEMENT.md](../../../ra2-rust-game-docs/FOOTCLASS_PATHFINDING_AND_MOVEMENT.md) — locomotor + zone integration
- **gamemd.exe addresses verified this session:**
  - `FootClass::Scan_For_Tiberium` @ 0x4DD0A0 — diamond-spiral scan, per-candidate predicate
  - `FootClass::Is_Cell_Harvestable` @ 0x4DCE80 — predicate body; calls Can_Reach_Zone, LandType==5, Can_Enter_Cell
  - `MapClass::Can_Reach_Zone` — zone-connectivity reachability check
- **INI keys:** none new. Existing `TiberiumShortScan`, `TiberiumLongScan` continue to drive radii via `MinerConfig::from_general_rules`.
- **Related Rust code:**
  - [src/sim/miner/miner_system.rs:209](../../../src/sim/miner/miner_system.rs#L209) — `handle_search_ore` (the file we change most)
  - [src/sim/miner/miner_system.rs:867](../../../src/sim/miner/miner_system.rs#L867) — `search_local_ore` (signature change)
  - [src/sim/production/production_economy.rs:34](../../../src/sim/production/production_economy.rs#L34) — `pick_best_resource_node` (signature change)
  - [src/sim/pathfinding/zone_map.rs:288](../../../src/sim/pathfinding/zone_map.rs#L288) — `ZoneGrid::can_reach` (the API we consume)
  - [src/sim/world/mod.rs:179](../../../src/sim/world/mod.rs#L179) — `Simulation.zone_grid: Option<ZoneGrid>`
  - [src/sim/movement/locomotor.rs:170](../../../src/sim/movement/locomotor.rs#L170) — `Locomotor.movement_zone`
  - [src/sim/slave_miner.rs:190, 333, 615](../../../src/sim/slave_miner.rs#L190) — three existing callers updated to pass `None`
  - [src/sim/miner/miner_tests.rs:955, 989](../../../src/sim/miner/miner_tests.rs#L955) — two existing test callers updated to pass `None`
