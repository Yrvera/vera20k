# Bridge PathGrid Runtime Refresh — Design

## Goal

Wire the existing `BridgeRuntimeState` collapse signal through to the app-side
`PathGrid` so A* sees destroyed bridges as non-traversable starting the tick
after collapse — matching gamemd's one-tick-delayed visibility.

## Architecture Context

The collapse pipeline on the sim side is already complete and correct:

- `combat::tick_combat_with_fog` emits `BridgeDamageEvent`s into
  `CombatResult.bridge_damage_events`.
- `world::advance_tick` (`src/sim/world/mod.rs:1241-1246`) calls
  `bridge_orchestrator::apply_bridge_damage_events`, which runs the 4-path
  dispatcher (HighSM → LowSM → LowDirect → HighDirect), kills ground occupants
  via `BlowUpBridge` semantics, drops in deck units, spawns debris with
  parity-correct RNG draw order, and calls `refresh_endpoint_active_flags()`
  to deactivate the affected bridge zone records.
- `BridgeRuntimeState::is_bridge_walkable(rx, ry)`
  ([`src/sim/bridge_state/mod.rs:732`](../../src/sim/bridge_state/mod.rs#L732))
  returns `false` once a cell's `damage_state` is `Destroyed`.
- `PathGrid::from_resolved_terrain_with_bridges`
  ([`src/sim/pathfinding/core.rs:976-1025`](../../src/sim/pathfinding/core.rs#L976-L1025))
  already projects `bridge_state` into per-cell `ground_walkable` (reverts to
  underlying terrain on destroyed deck), `bridge_walkable` (clears on
  destruction), and `transition` (clears bridgehead transition when the
  adjacent body span collapses). All bit-0x80 / bit-0x100 / bit-0x200 parity
  logic from `SetBridgeDirection` lives in this function.

The bug is one layer up. `state.path_grid` (the grid A* actually consumes)
lives in `AppState`, rebuilt by
[`app_sim_tick::rebuild_dynamic_path_grid`](../../src/app_sim_tick.rs#L701)
which **clones `state.path_grid_base`** — a static cache built once at map
load via the bridgeless `PathGrid::from_resolved_terrain`
([`app_init.rs:667-669`](../../src/app_init.rs#L667-L669)) — then stamps
building footprints and wall overlays on top.

Result: `state.path_grid` reflects intact-bridge walkability forever. The
orchestrator's `refresh_bridge_zones_if_dirty` already calls
`from_resolved_terrain_with_bridges` with current `bridge_state` to feed
`rebuild_zone_grid`, but that fresh grid is discarded after the zone rebuild
— never installed as the A* grid.

The `refresh_after_tick` trigger pattern in `app_sim_tick.rs` already exists
for `destroyed_structure`, `spawned_entities`, `ownership_changed`, and
overlay-passability dirty cells. Bridge collapse is the only refresh trigger
not wired through it.

## Impact Analysis

**Touched files (4):**

1. [`src/sim/world/mod.rs`](../../src/sim/world/mod.rs)
   - `TickResult`: add `pub bridge_state_changed: bool`.
   - `advance_tick`: receive bool return from
     `apply_bridge_damage_events`, propagate to `TickResult`.

2. [`src/sim/world/bridge_orchestrator.rs`](../../src/sim/world/bridge_orchestrator.rs)
   - Change `apply_bridge_damage_events` return type from `Vec<u64>` to
     `(Vec<u64>, bool)` (or wrap the bool into a result struct). Bool = "any
     `StateOutcome::Collapsed` was produced this batch".
   - Keep `refresh_bridge_zones_if_dirty` and its in-tick zone rebuild
     intact — AI in Phase 8 needs fresh zones within the same tick.

3. [`src/app_sim_tick.rs`](../../src/app_sim_tick.rs)
   - `rebuild_dynamic_path_grid`: replace `base_grid.clone()` with a fresh
     `PathGrid::from_resolved_terrain_with_bridges(terrain,
     sim.bridge_state.as_ref())` call. Stamp building footprints + walls on
     top as before.
   - After tick result handling block (`~line 506`): add
     `if tick_result.bridge_state_changed { refresh_after_tick = true; }`.

4. [`src/sim/world/world_tests.rs`](../../src/sim/world/world_tests.rs)
   - Promote `test_bridge_damage_rebuilds_path_grid` into an integration test
     that exercises the new TickResult flag path end-to-end.

**Risk areas:**

- **`path_grid_base` becomes dead.** Field stays populated by `app_init`
  but is no longer read by the hot path. Leave it in place for now to keep
  the diff focused — clean up as a follow-up.
- **Zone rebuild double-count.** Orchestrator still calls `rebuild_zone_grid`
  inside `advance_tick` (for in-tick AI consumers). `rebuild_dynamic_path_grid`
  also calls `sim.rebuild_zone_grid(&grid)` after installing the new
  `state.path_grid` ([`app_sim_tick.rs:754-758`](../../src/app_sim_tick.rs#L754-L758)).
  On a bridge-collapse tick, zones rebuild twice — once in-tick, once
  post-tick. The second rebuild uses the same `from_resolved_terrain_with_bridges`
  output, produces the same zone graph, and updates `sim.prev_path_grid` so
  next-tick diffs are correct. Cost: bounded; correctness: preserved.
- **`refresh_endpoint_active_flags` is one-way.** Repair will need a
  bidirectional sibling. Out of scope here.
- **Determinism.** `path_grid` is `#[serde(skip)]`-equivalent (app-side, not
  in state hash). The new `bridge_state_changed` flag is a `TickResult`
  field — observable artifact, not hashed state. No determinism impact.
- **Parallel sessions / CI.** Approach modifies a return type
  (`apply_bridge_damage_events`). One internal caller (`world::advance_tick`
  line 1241) and existing tests are the only call sites. Low blast radius.

## Chosen Approach

**App-side signal + always-rebuild-from-terrain.**

The orchestrator returns whether any collapse occurred this batch. `advance_tick`
threads that through `TickResult.bridge_state_changed`. The app sets
`refresh_after_tick = true` on this flag (same as the four existing triggers).
`rebuild_dynamic_path_grid` is changed to build the base grid via
`PathGrid::from_resolved_terrain_with_bridges(terrain, sim.bridge_state.as_ref())`
on every refresh — replacing the static `state.path_grid_base.clone()`.

This re-uses the projection logic that's already correct (all
`SetBridgeDirection` bit-write parity is already inside
`from_resolved_terrain_with_bridges`) and matches the existing
refresh-trigger pattern bit-for-bit. The one-full-map iteration per refresh
tick is bounded (64²–256² cells, rare ticks) and replaces an equally-sized
clone.

Rejected:
- **Sim-owned PathGrid** — multi-day ownership refactor; doesn't improve
  parity, just architecture.
- **Per-cell delta patch** — would duplicate the SetBridgeDirection
  neighbor logic outside `from_resolved_terrain_with_bridges`; two code
  paths for one invariant.

## Tiny-Detail Ledger

The implementation must preserve all of the following observable behaviors.
Every item is **already implemented inside
`PathGrid::from_resolved_terrain_with_bridges`** — this design's
responsibility is to keep using that function, not reinvent it.

1. **Destroyed bridge cells stop being walkable on the bridge layer the
   tick AFTER collapse, not the same tick.**
   `[doc: BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md §5]` —
   "ProcessBridgeDamageStateMachine_* runs AFTER movement, so destroyed-cell
   visibility is one tick delayed." Preserved by the existing
   advance_tick → return → app-refresh ordering. Movement in Phase 1 of tick
   T runs against the old grid; bridge collapse fires in Phase 5 of tick T;
   app rebuilds grid after `advance_tick` returns; Phase 1 of tick T+1 sees
   the new grid.

2. **Collapse affects more than the destroyed cell.** `SetBridgeDirection`
   visits 5-6 cells and clears bit 0x100 (`bridge_walkable`) on cells 1-3,
   and clears bit 0x200 (`transition` / bridgehead) on cells 1-2.
   `[doc: §3.4 Cells visited table]`. Preserved because the design rebuilds
   from terrain+bridge_state across the whole map, not just at the impact
   cell.

3. **Bridgehead cells (bit 0x200) gate Ground↔Bridge transitions.** Losing
   this bit on collapse means no transition into the dead span.
   `[doc: §3.1 — diff-4 entry branch]`. Already enforced in
   [`from_resolved_terrain_with_bridges:1009-1011`](../../src/sim/pathfinding/core.rs#L1009-L1011)
   via the `state.is_bridge_walkable` gate on `bridge_transition`.

4. **Ground walkability under a destroyed deck reverts to the underlying
   terrain.** Cliff/water below stays unwalkable; clear ground becomes
   walkable. Already enforced in
   [`from_resolved_terrain_with_bridges:990-1005`](../../src/sim/pathfinding/core.rs#L990-L1005).

5. **Bit 0x80 origin is map-load only.** Only `SetBridgeDirection` (collapse/
   repair) and `OverlayClass::Mark` (load) write it; no other gameplay
   mutations. `[doc: §Q1 RESOLVED]`. Our `bridge_state`-driven projection is
   the correct model.

6. **Bits 0x2000 / 0x4000 are write-only TS-legacy.** Safe to ignore.
   `[doc: §Q2 RESOLVED]`.

7. **Zone graph deactivation fires on the first destroyed cell of a group,
   not whole-group destruction.**
   `[code: bridge_state/mod.rs:1131-1147 refresh_endpoint_active_flags]`.
   Already correct — orchestrator calls this in
   `refresh_bridge_zones_if_dirty`.

8. **Repair must reverse #1-3 symmetrically.** `refresh_endpoint_active_flags`
   is one-way today. **Out of scope here.** When BridgeRepairHut state-machine
   lands, that function will need a bidirectional sibling. The PathGrid
   refresh pipeline this design adds is already symmetric — once
   `damage_state` flips back to `Healthy`, `is_bridge_walkable` returns true
   and the same dirty-flag plumbing re-installs walkable cells.

9. **Deck and ground are separate layers in the A* graph.** Projection from
   `bridge_state` flips bridge-layer walkability without touching ground-
   layer fields beyond #4.
   `[code: from_resolved_terrain_with_bridges:976-1025]`.

10. **One-tick delay is preserved by the architecture, not by explicit code.**
    The signal fires inside `advance_tick`; the rebuild fires after
    `advance_tick` returns; the next `advance_tick` consumes the rebuilt
    grid. No frame-skip logic needed.

## Design

### Components

- `bridge_orchestrator::apply_bridge_damage_events` — returns
  `BridgeOrchestratorResult { despawned_ids: Vec<u64>, state_changed: bool }`
  (or a `(Vec<u64>, bool)` tuple — pick during implementation). `state_changed`
  is true iff at least one `StateOutcome::Collapsed` was produced.
- `TickResult::bridge_state_changed: bool` — new field, populated by
  `world::advance_tick` from the orchestrator's return.
- `rebuild_dynamic_path_grid` — change base from `base_grid.clone()` to
  `PathGrid::from_resolved_terrain_with_bridges(terrain, sim.bridge_state.as_ref())`.

### Interfaces / Contracts

**Signal contract:**
`tick_result.bridge_state_changed == true` ⇒ at least one bridge cell in
the sim transitioned to `DamageState::Destroyed` (or any future
non-`Destroyed → Healthy` transition that affects walkability). The app
MUST trigger a path grid rebuild in response.

**Build contract:**
`rebuild_dynamic_path_grid` MUST build its base from terrain+bridge_state
on every call. Building from a stale cached grid is forbidden.

**Tick-order contract:**
The bridge-state mutation happens inside `advance_tick`. The path grid
refresh happens between `advance_tick` returning and the next
`advance_tick` being called. No same-tick read of the new grid is allowed.

### Data Flow

```
combat::tick_combat_with_fog
  └─> CombatResult.bridge_damage_events
        ↓
world::advance_tick Phase 5
  └─> bridge_orchestrator::apply_bridge_damage_events
        ├─> dispatcher emits StateOutcome::Collapsed
        ├─> BridgeRuntimeState.cell.damage_state = Destroyed
        ├─> refresh_endpoint_active_flags (one-way deactivation)
        ├─> in-tick zone rebuild (for same-tick AI)
        └─> returns (Vec<u64> despawned, bool state_changed)
              ↓
world::advance_tick
  └─> TickResult { bridge_state_changed: state_changed, ... }
        ↓
app_sim_tick (after advance_tick returns)
  └─> if tick_result.bridge_state_changed { refresh_after_tick = true; }
        ↓
        if refresh_after_tick { rebuild_dynamic_path_grid(state); }
              ↓
rebuild_dynamic_path_grid
  ├─> let mut grid = PathGrid::from_resolved_terrain_with_bridges(
  │       terrain, sim.bridge_state.as_ref())
  ├─> stamp building footprints
  ├─> stamp wall overlays
  ├─> state.path_grid = Some(grid)
  └─> sim.rebuild_zone_grid(&grid)
              ↓
Next advance_tick consumes the rebuilt grid; Phase 1 movement sees new walkability.
```

### Error Handling

- `sim.resolved_terrain` is `None` (uninitialized state): early-return from
  `rebuild_dynamic_path_grid`. Same as current code's guards.
- `sim.bridge_state` is `None` (map has no bridges):
  `from_resolved_terrain_with_bridges(terrain, None)` falls back to
  `cell.bridge_walkable` from the resolved terrain — equivalent to the
  current `path_grid_base` behavior. No change in walkability.

### Testing Strategy

1. **Unit-promote `test_bridge_damage_rebuilds_path_grid`** into an
   end-to-end test:
   - Build a sim with a 3-cell EW high bridge.
   - Step one tick, capture `state.path_grid` walkability — all 3 cells
     walkable on bridge layer.
   - Inject a `BridgeDamageEvent` via the standard combat path.
   - Step one more tick. Assert `tick_result.bridge_state_changed == true`,
     post-tick PathGrid has all 3 cells **not walkable** on bridge layer.

2. **Refresh trigger fan-in test**: verify that combining
   `bridge_state_changed` with `destroyed_structure` in the same tick still
   triggers a single rebuild and produces the correct walkability for both
   the destroyed building's foundation and the destroyed bridge cells.

3. **No-collapse no-rebuild test**: a tick where combat fires at a bridge
   but the BridgeStrength RNG gate blocks the collapse — assert
   `tick_result.bridge_state_changed == false` and the grid is not rebuilt
   (observable via a rebuild-counter or by checking `state.path_grid` is
   `Some(_)` but unchanged).

4. **Bridge-adjacent cell coverage** (regression): destroy one body cell
   of a bridge, assert that the bridgehead cell adjacent to the collapsed
   body loses its `transition` flag in the rebuilt grid (per ledger #2/#3).
   This proves the design isn't accidentally per-cell; the rebuild walks
   the whole grid.

5. **Determinism**: state-hash test verifying that the bridge collapse →
   refresh sequence produces the same state hash on two parallel sim runs
   with the same inputs. The hash itself doesn't change from this work
   (path_grid is app-side), but the test guards against accidental
   sim-side state writes during the orchestrator return-type refactor.

## Architectural Decisions

**Patterns followed:**
- Mirrors the existing `tick_result.destroyed_structure` /
  `spawned_entities` / `ownership_changed` → `refresh_after_tick` pattern in
  `app_sim_tick.rs`. The bridge signal is a new entry in that same fan-in.
- Preserves the sacred `sim/` → `app` boundary. PathGrid stays owned by the
  app; sim signals via `TickResult` only.
- Re-uses `PathGrid::from_resolved_terrain_with_bridges` as the single
  source of truth for "how does bridge state become PathGrid state."

**Patterns deviated from:**
- Drops `state.path_grid_base` as a hot-path cache. The field stays
  populated for now (no caller-side cleanup in this diff), but
  `rebuild_dynamic_path_grid` no longer reads it. **Justification:** the
  cache was the bug. A static-load grid cannot reflect runtime bridge
  state, and there is no incremental way to update it that doesn't
  duplicate the projection logic. Full rebuild is bounded-cost
  (64²–256² cells per refresh tick) and parity-correct.

**Tech debt introduced:**
- `state.path_grid_base` field is dead. **Plan to address:** follow-up
  cleanup PR to remove the field, the app_init code that populates it, and
  the `path_grid_base` plumbing through `app_transitions.rs`. Tracked but
  not blocking this fix.

**Known follow-ups (NOT in scope):**
- BridgeRepairHut state machine — when implemented, `BridgeRuntimeState`
  needs a damage_state reverse path (Damaged/PartialCollapse → Healthy) and
  `refresh_endpoint_active_flags` needs a bidirectional sibling. The
  PathGrid plumbing here is symmetric and will work for repair the moment
  those upstream pieces land.
- `path_grid_base` field cleanup (described above).

## Alternatives Considered

**Approach B: Sim owns `path_grid`.** Move `state.path_grid` into
`Simulation`; sim refreshes inside `advance_tick`. Rejected: pure ownership
refactor that touches every site that currently passes `&PathGrid` from
the app. Wall-overlay stamping (currently in `rebuild_dynamic_path_grid`,
reading `state.overlays` + `state.overlay_registry`) would have to move
into sim — those structures live in the app layer. Multi-day refactor
for no parity improvement.

**Approach C: Per-cell delta patch.** Orchestrator emits the set of
cells whose walkability changed; `rebuild_dynamic_path_grid` clones
`path_grid_base` and patches only those cells. Rejected: requires
replicating `SetBridgeDirection`'s 5-6 cell neighbor pattern outside
`from_resolved_terrain_with_bridges`, creating two code paths for one
invariant. High regression risk for marginal performance gain on
already-bounded full-rebuild cost.
