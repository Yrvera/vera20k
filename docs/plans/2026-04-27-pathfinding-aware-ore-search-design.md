# Pathfinding-Aware Ore Search — Design

## Goal

Filter ore-search candidates by zone-based reachability so harvesters never target ore they can't physically reach (matching gamemd's `Is_Cell_Harvestable` → `Can_Reach_Zone` predicate).

## Architecture Context

### Current ore-search flow

[handle_search_ore](../../src/sim/miner/miner_system.rs#L209) runs three rings, returning the first hit:

1. **Local continuation** — [search_local_ore](src/sim/miner/miner_system.rs#L867) within `local_continuation_radius` (6 cells) around `last_harvest_cell`.
2. **Archive target** — if `last_harvest_cell` still has ore, return there.
3. **Long scan** — `search_local_ore` within `long_scan_radius` (48 cells) around the harvester's current cell.
4. **Global fallback** — [pick_best_resource_node](src/sim/production/production_economy.rs#L34) picks the nearest ore anywhere on the map (unbounded).
5. **No ore** — `WaitNoOre`.

`search_local_ore` and `pick_best_resource_node` filter only on `radius` and `remaining > 0`. No reachability check. The path grid is passed to `handle_search_ore` as `_path_grid: Option<&PathGrid>` (parameter is unused).

### Available zone infrastructure

- [`Simulation.zone_grid: Option<ZoneGrid>`](src/sim/world/mod.rs#L179) — built at terrain load, incrementally updated on building placement/destruction.
- [`ZoneGrid::can_reach(mz, from, from_layer, to, to_layer) -> bool`](src/sim/pathfinding/zone_map.rs#L288) — O(1) reachability via union-find super-zones. Already wired up; the gamemd `Can_Reach_Zone` equivalent.
- Returns `true` when `zone_grid` is missing (conservative no-op for tests).

### gamemd structure (verified via Ghidra this session)

`FootClass::Scan_For_Tiberium` @ 0x4DD0A0 is a diamond-spiral scan; per candidate cell it calls `FootClass::Is_Cell_Harvestable` @ 0x4DCE80, which gates on `Can_Reach_Zone(unit_zone, target_cell)` along with playfield/shroud/LandType/Can_Enter_Cell checks. The filter is a **per-candidate predicate inside the search loop** — not pre-filter, not post-filter.

### The Tiberium-cell zone problem

Tiberium cells are marked impassable in the path grid (so A* doesn't path through ore fields). Consequently `zone_at(ore_cell)` returns `ZONE_INVALID` for every ore cell. A naive `can_reach(harvester, ore_cell)` always returns false.

The reachability test must therefore use the ore cell's **passable neighbor**, not the ore cell itself: "is there at least one passable cell adjacent to the ore that the harvester can reach?"

## Impact Analysis

**Touches:**

- [src/sim/miner/miner_system.rs](src/sim/miner/miner_system.rs) — `tick_miners` (thread `&ZoneGrid` down), `handle_search_ore` (compute harvester effective zone, build filter), `search_local_ore` (accept filter closure).
- [src/sim/production/production_economy.rs](src/sim/production/production_economy.rs) — `pick_best_resource_node` (accept filter closure).

**Doesn't touch:**

- `handle_move_to_ore`, `handle_harvest`, `handle_return`, dock sequence — out of scope.
- Slave miner ore search ([slave_miner.rs](src/sim/slave_miner.rs)) — has its own search; not in scope. Slave-miner can opt into the filter later if a similar bug surfaces.

**Determinism:** all reads. No new state, no RNG, no float math. `can_reach` is a deterministic structure lookup.

**Tick ordering:** unchanged. Search runs at the existing `tick_miners` slot; `zone_grid` updates run at building place/destroy and are committed before the next tick.

**Snapshot serialization:** no struct-field changes; nothing new to serialize.

**Risk areas:**

- Tests in [miner_tests.rs](src/sim/miner/miner_tests.rs) construct `PathGrid::new(64,64)` and never build a `ZoneGrid`. They'll pass `None` for the zone grid; the closure builder returns `None`; the search runs unfiltered (current behavior). No test breakage.
- Harvester standing on Tiberium with all 8 immediate neighbors also Tiberium: probe returns `None` → filter is `None` for that tick → search runs unfiltered. Acceptable degenerate case (one tick of unfiltered search before perimeter ore extracts).
- `ZoneGrid` is `None` on non-skirmish setups: filter is `None` → unfiltered search. Same conservative degradation as `can_reach` itself.

## Chosen Approach

Pass an optional filter closure into `search_local_ore` and `pick_best_resource_node`. The closure encapsulates the reachability test against `ZoneGrid`. This mirrors gamemd's per-candidate-predicate structure and keeps the search functions reachability-agnostic.

### Why not the alternatives

- **Filter at the body with a concrete `&ZoneGrid` parameter** — couples the search functions to the zone module unnecessarily. Closure-as-predicate generalises to future filters (blacklists, alliance, etc.).
- **Pre-filter the resource_nodes BTreeMap once per scan** — O(N) over all map ore on every search; doesn't scale to the project's 20k-units / 30-players target.

## Design

### Components

No new structs, modules, or enums. Two existing fns gain an optional filter parameter; one call site builds the closure.

### Interfaces / Contracts

```rust
// src/sim/miner/miner_system.rs
pub(crate) fn search_local_ore(
    nodes: &BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,  // NEW
) -> Option<(u16, u16)>;

// src/sim/production/production_economy.rs
pub fn pick_best_resource_node(
    nodes: &BTreeMap<(u16, u16), ResourceNode>,
    from: (u16, u16),
    filter: Option<&dyn Fn((u16, u16)) -> bool>,  // NEW
) -> Option<(u16, u16)>;
```

Existing callers pass `None` for the filter — observable behavior unchanged. The miner system passes a real closure.

### Data Flow

```
tick_miners(sim, rules, config, path_grid, zone_grid)   // zone_grid: Option<&ZoneGrid> NEW
  └─ handle_search_ore(sim, config, path_grid, zone_grid, snap)
        ├─ harvester_mz = entity.locomotor?.movement_zone   (default Normal)
        ├─ harvester_layer = entity.movement_layer_or_ground()
        ├─ harvester_zone_cell = effective_zone_cell(zone_grid, mz, snap.rx, snap.ry)
        │   └─ probe self + 8 neighbors, return first cell with valid zone
        ├─ filter = zone_grid.zip(harvester_zone_cell).map(|(zg, ref_cell)| {
        │       move |ore_cell| ore_reachable(zg, mz, layer, ref_cell, ore_cell)
        │   })
        ├─ try search_local_ore(short, center=last_harvest, filter)
        ├─ try archive (with filter applied to the archive cell too)
        ├─ try search_local_ore(long, center=current, filter)
        ├─ try pick_best_resource_node(global, filter)
        └─ WaitNoOre
```

### Reachability predicate

```rust
fn ore_reachable(
    zone_grid: &ZoneGrid,
    mz: MovementZone,
    layer: MovementLayer,
    harvester_zone_cell: (u16, u16),
    ore_cell: (u16, u16),
) -> bool {
    // Ore cells are ZONE_INVALID (Tiberium blocked in path grid).
    // Reachable iff some passable neighbor of the ore cell is in
    // the harvester's connected zone component.
    for (dx, dy) in NEIGHBORS_8 {
        let nx = ore_cell.0.checked_add_signed(dx);
        let ny = ore_cell.1.checked_add_signed(dy);
        if let (Some(nx), Some(ny)) = (nx, ny) {
            if zone_grid.can_reach(mz, harvester_zone_cell, layer, (nx, ny), layer) {
                return true;
            }
        }
    }
    false
}
```

### Effective-zone-cell probe (Q2 = Approach A)

```rust
fn effective_zone_cell(
    zone_grid: &ZoneGrid,
    mz: MovementZone,
    rx: u16,
    ry: u16,
) -> Option<(u16, u16)> {
    let zone_map = zone_grid.map_for(mz)?;
    // Try the harvester's cell first.
    if zone_map.zone_at(rx, ry, MovementLayer::Ground) != ZONE_INVALID {
        return Some((rx, ry));
    }
    // Probe 8 neighbors.
    for (dx, dy) in NEIGHBORS_8 {
        let nx = rx.checked_add_signed(dx)?;
        let ny = ry.checked_add_signed(dy)?;
        if zone_map.zone_at(nx, ny, MovementLayer::Ground) != ZONE_INVALID {
            return Some((nx, ny));
        }
    }
    None
}
```

If both the harvester's cell and all 8 neighbors are `ZONE_INVALID`, return `None`. The caller then passes `None` for the filter — search degrades to current (unfiltered) behavior for that tick. The harvester will harvest its current cell, opening up perimeter neighbors next tick.

### Threading the parameter

`tick_miners` currently takes `Option<&PathGrid>`. Extend its signature to also take `Option<&ZoneGrid>`. Update the single dispatch caller in [tick_resource_economy](src/sim/production/production_economy.rs#L13) to pass `sim.zone_grid.as_ref()`.

### Error Handling

No new error paths. All new lookups return `Option`/`bool`; missing zone data conservatively skips filtering.

### Testing Strategy

Three new unit tests in [miner_tests.rs](src/sim/miner/miner_tests.rs):

1. **`unreachable_ore_filtered_out`** — build a `ZoneGrid` where ore is in a disconnected zone from the harvester. Assert `handle_search_ore` does not pick it; the harvester transitions to `WaitNoOre` if no reachable ore exists.

2. **`reachable_ore_picked_over_closer_unreachable`** — disconnected nearby ore + connected farther ore. Assert the farther reachable cell is picked.

3. **`harvester_on_tiberium_falls_back_to_unfiltered`** — harvester standing on a Tiberium cell with passable neighbors. Assert the probe finds an effective zone via a neighbor and the filter still applies (i.e., this is the mid-harvest re-search case working correctly).

Existing tests don't construct a `ZoneGrid`, so the filter is `None` and search runs unfiltered. No existing test should break.

## Architectural Decisions

**Patterns followed:**
- Closure-as-predicate, matching the existing pattern in [pathfinding::core search options](src/sim/pathfinding/core.rs).
- Optional filter parameter with `None`-degrades-to-no-op semantics, matching `can_reach`'s own missing-zone-data behavior.
- Sim-only change; no render/UI/audio touch.

**Patterns deviated from:** none.

**Tech debt:**

- The Rust `search_local_ore` is a bounded-box scan picking best across the radius; gamemd's `Scan_For_Tiberium` is a diamond spiral with ring-by-ring early-exit. After this fix, the picked cell may still differ from gamemd's pick when ore density varies significantly across the radius. Flagged as LOW-severity in the disparity scan; deferred to a separate brainstorm.

- `pick_best_resource_node` becomes effectively reachable-only after this change. If a player ever wants to "go to the nearest ore even if unreachable" (debug command, AI behavior, etc.), the unfiltered call is still available by passing `None`.

## Alternatives Considered

- **Approach 2 (concrete `&ZoneGrid` parameter inside the search body)** — rejected: couples search to the zone module unnecessarily; closure form is more general.
- **Approach 3 (pre-filter the resource_nodes map once per scan)** — rejected: O(N) over all map ore per search; doesn't scale.
- **Track `last_passable_zone` on the Miner struct** — rejected for Q2: new state field, more bookkeeping, must be in determinism hash; the 9-cell probe gives the same answer with no state.
- **Skip the filter when harvester is on Tiberium** — rejected for Q2: introduces an asymmetry between mid-harvest and post-undock re-search that's avoidable.
- **Bundle the spiral structural fix into this change** — rejected: spiral-vs-box is LOW severity and orthogonal to the head-butt bug; bundling makes the change harder to review and test.

## Definition of Done

- [ ] `search_local_ore` and `pick_best_resource_node` accept an optional filter closure.
- [ ] `handle_search_ore` builds a reachability filter from `ZoneGrid` + harvester effective zone.
- [ ] `tick_miners` threads `&ZoneGrid` from the `Simulation`.
- [ ] Three new unit tests pass.
- [ ] Existing miner tests still pass unmodified.
- [ ] `cargo clippy` clean for the changed files.
- [ ] Manual in-game verification: chrono miner with refinery placed adjacent to back-side ore exits, scans, picks reachable ore, no head-butt cycle observed.
