# Bridge PathGrid Runtime Refresh — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Wire the existing `BridgeRuntimeState` collapse signal through to the app-side `PathGrid` so A* sees destroyed bridges as non-traversable starting the tick after collapse.

**Architecture:** The collapse pipeline is already complete on the sim side and correctly mutates `BridgeRuntimeState`. `PathGrid::from_resolved_terrain_with_bridges` already projects that state into per-cell walkability. The fix is to (a) signal "bridge state changed" from sim to app via a new `TickResult` flag, and (b) change `rebuild_dynamic_path_grid` to build from terrain+bridge_state instead of cloning a stale base cache.

**Design Doc:** [docs/plans/2026-05-12-bridge-pathgrid-runtime-refresh-design.md](2026-05-12-bridge-pathgrid-runtime-refresh-design.md)

---

## Grounding Summary

- **Docs (R1):** [`ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md) — §3.4 bit-write map, §3.5 caller graph, §5 tick ordering, §6 status, §11 recommendations. HIGH confidence on every load-bearing claim.
- **Ghidra (R2):** No new binary lookups required. Design re-uses verified findings: 0x47E040 SetBridgeDirection, §3.4 cell-visit pattern (5-6 cells per collapse), §5 one-tick-delayed visibility ("ProcessBridgeDamageStateMachine_* runs AFTER movement").
- **Repo pattern (R3):** `TickResult.destroyed_structure` → `refresh_after_tick` → `rebuild_dynamic_path_grid` ([`app_sim_tick.rs:506-513`](../../src/app_sim_tick.rs#L506-L513), [`app_sim_tick.rs:629-630`](../../src/app_sim_tick.rs#L629-L630)). New `bridge_state_changed` flag mirrors this pattern.
- **INI (R4):** No INI keys involved. `BridgeStrength` / `DestroyableBridges` already consumed by the orchestrator's outer gate and RNG dispatch.
- **Git state:** All 4 touched files have recent unrelated commits; none invalidate the design premise. `apply_bridge_damage_events` still returns `Vec<u64>`, `rebuild_dynamic_path_grid` still clones `state.path_grid_base`, `from_resolved_terrain_with_bridges` projection logic is intact.
- **Unknowns:** None. Implementation surface fully specified.

## Key Technical Decisions

- **Decision:** Replace `apply_bridge_damage_events` return type from `Vec<u64>` to `bool` (state_changed). **Confidence:** high.
  - **Source:** Orchestrator docstring at [`bridge_orchestrator.rs:46-50`](../../src/sim/world/bridge_orchestrator.rs#L46-L50) explicitly says "Callers should not yet use the return value" and the only caller binds to `_bridge_fallout_ids`. Tests bind to `let _despawned = ...`. Forward-compatibility cost is zero; if a future caller needs IDs, they add a struct return.

- **Decision:** Drop `state.path_grid_base.clone()` in favor of full rebuild via `from_resolved_terrain_with_bridges` on every refresh. **Confidence:** high.
  - **Source:** Design doc §"Chosen Approach"; per-cell delta and ownership-refactor alternatives both rejected. `from_resolved_terrain_with_bridges` is the single source of truth for cell-projection logic ([`core.rs:976-1025`](../../src/sim/pathfinding/core.rs#L976-L1025)).

- **Decision:** `path_grid_base` field stays populated by `app_init` but becomes dead on the hot path. **Confidence:** high.
  - **Source:** Design doc §"Tech debt introduced". Cleanup is explicit follow-up. Reduces diff scope and keeps unrelated `app_init` / `app_transitions` plumbing untouched.

- **Decision:** Keep in-tick zone rebuild inside `bridge_orchestrator::refresh_bridge_zones_if_dirty`. **Confidence:** high.
  - **Source:** AI in `advance_tick` Phase 8 ([`world/mod.rs:1424-1451`](../../src/sim/world/mod.rs#L1424-L1451)) reads zones; deferring zone rebuild to post-`advance_tick` would leak stale zones to AI same-tick. Design doc §"Impact Analysis" risk #2.

## Open Questions

### Resolved During Planning

- **Q: Should the orchestrator return a struct or a primitive bool?** Resolved: bool. Existing `Vec<u64>` return is documented as "do not use" and only bound to discard. YAGNI on a forward-compatible struct.
- **Q: Does the in-tick zone rebuild cause double work?** Resolved: yes, by ~one full-map iteration on collapse ticks. Acceptable trade — same-tick AI consumers need fresh zones, and the cost is bounded.
- **Q: Does any other tick-flag consumer (sound, animation, defeat detection) read `path_grid`?** Resolved: no. Search for `state.path_grid` consumers shows only the next `advance_tick` invocation reads it. Mid-frame UI hit-tests read `state.path_grid` but on the post-rebuild value, which is correct.

### Deferred to Implementation

- **Whether wall-overlay stamping inside `rebuild_dynamic_path_grid` needs adjustment after the base swap.** The current code stamps walls AFTER cloning the base ([`app_sim_tick.rs:736-747`](../../src/app_sim_tick.rs#L736-L747)). With the new base from `from_resolved_terrain_with_bridges`, walls still need to be stamped — but the new base may already block some cells the walls would block. Verify in Task 4 that the order still produces identical wall blocking. If overlap detected, no behavior change is needed (blocking idempotent), but a log warning may be useful.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [`src/sim/world/mod.rs`](../../src/sim/world/mod.rs) | Add `bridge_state_changed` field to `TickResult`; propagate from orchestrator |
| Modify | [`src/sim/world/bridge_orchestrator.rs`](../../src/sim/world/bridge_orchestrator.rs) | Change `apply_bridge_damage_events` return type to `bool` |
| Modify | [`src/app_sim_tick.rs`](../../src/app_sim_tick.rs) | Trigger `refresh_after_tick` on `bridge_state_changed`; rebuild grid from terrain+bridge_state |
| Modify | [`src/sim/world/world_tests.rs`](../../src/sim/world/world_tests.rs) | Update existing `test_bridge_damage_rebuilds_path_grid` for new flag; add no-collapse and bridge-adjacent regression tests |

## Interface Changes

**Modified public-ish API:**

- `bridge_orchestrator::apply_bridge_damage_events` return type:
  `Vec<u64>` → `bool`. **Depends on:** [`world/mod.rs:1241`](../../src/sim/world/mod.rs#L1241) (only caller). **Risk:** trivial — only caller discards the value today.

- `TickResult` gains a new public field `bridge_state_changed: bool`. **Depends on:** every `TickResult` consumer. **Risk:** low — `TickResult` is constructed in one place ([`world/mod.rs:1485-1493`](../../src/sim/world/mod.rs#L1485-L1493)) and read in app_sim_tick.rs. Rust struct-init exhaustiveness will catch missed sites at compile time.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (no math added; this is plumbing)
- [x] New state included in deterministic state hash — `bridge_state_changed` is a `TickResult` field, not Simulation state. Not part of state hash. The mutation it signals (BridgeRuntimeState `damage_state`) is already in the state hash via the orchestrator's existing pipeline.
- [x] No dependencies on render/ui/sidebar/audio/net — sim code only references `crate::sim::*`. `app_sim_tick.rs` is app-side and consumes the signal; no reverse dependency.
- [x] Tick ordering impact noted — see design doc §"Impact Analysis" risk #2. Orchestrator runs in Phase 5 (combat); flag set; `TickResult` returned; app rebuilds grid post-return; next tick's Phase 1 movement sees new grid. Matches gamemd's one-tick-delayed visibility.
- [x] BTreeMap iteration order considered — N/A (no map iteration added)

## Risk Areas

From design doc §"Impact Analysis":

1. **Zone double-rebuild on collapse ticks** — acceptable cost; no correctness impact.
2. **`refresh_endpoint_active_flags` is one-way** — explicit out-of-scope follow-up; documented in design as known limitation.
3. **`path_grid_base` field becomes dead** — explicit follow-up; non-blocking.
4. **Wall stamp order after base swap** — Task 4 must verify wall blocking is preserved after the base change. Deferred to implementation.
5. **`bridge_state` is `None`** — `from_resolved_terrain_with_bridges(terrain, None)` falls back to `cell.bridge_walkable` from resolved terrain, equivalent to current `path_grid_base` behavior. Safe.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | A* sees destroyed bridge cells as non-traversable starting tick T+1 after collapse on tick T | Player observable: units no longer walk onto thin air over collapsed bridges. Fires every match where a bridge is destroyed (C4, IonCannon, repair-then-collapse) | Integration test in Task 6: damage event → next tick assert all collapsed cells return `false` from `is_walkable_on_layer(.., Bridge)` |
| Task 4 | Bridgehead cells adjacent to a collapsed body span lose their `transition` flag (bit 0x200 equivalent) | Without this, A* still permits Ground→Bridge entry into the dead span. Player observable: unit walks onto bridgehead, then off into nothing. Per BRIDGE_DEFERRED_MECHANICS report §3.4 | Regression test in Task 7: destroy one body cell, assert adjacent bridgehead's `transition` is false in the rebuilt grid |
| Task 4 | Ground walkability under a destroyed deck reverts to underlying terrain (water/cliff below stays blocked; clear ground becomes walkable) | Player observable: collapse over water = no ground passage; collapse over clear ground = units can walk under | Already enforced in `from_resolved_terrain_with_bridges:990-1005`; Task 4 verifies it's reachable via the new build path |
| Task 5 | One-tick-delayed visibility preserved | Per BRIDGE_DEFERRED_MECHANICS §5: "ProcessBridgeDamageStateMachine_* runs AFTER movement, so destroyed-cell visibility is one tick delayed." Player perception: collapse animation plays this tick, next tick units start re-routing. | Integration test in Task 6 must observe the grid mid-tick (path unchanged) vs post-tick (path updated) |

---

## Tasks

### Task 1: Add `bridge_state_changed` field to `TickResult`

**Why:** Defines the signal contract before any code produces or consumes it. Pure type-definition task — surfaces struct-init exhaustiveness errors at every `TickResult` construction site so we can find them all in one compile.

**Files:**
- Modify: `src/sim/world/mod.rs:73-86` (TickResult struct definition)
- Modify: `src/sim/world/mod.rs:1485-1493` (TickResult construction in `advance_tick`)

**Pattern:** Mirror existing `destroyed_structure`, `spawned_entities`, `ownership_changed` fields.

**Step 1: Add the field to `TickResult`.**

In `src/sim/world/mod.rs`, modify the `TickResult` struct (currently at lines 73-86):

```rust
/// Result of one deterministic simulation tick.
#[derive(Debug, Clone, Copy)]
pub struct TickResult {
    pub tick: u64,
    pub executed_commands: usize,
    pub state_hash: u64,
    pub spawned_entities: bool,
    /// A structure was destroyed (combat, sell, crush) — PathGrid needs rebuild
    /// to unblock the footprint.
    pub destroyed_structure: bool,
    /// An entity's owner changed (garrison transfer, engineer capture) — sprite
    /// atlas needs rebuild for the new house color.
    pub ownership_changed: bool,
    /// A bridge cell transitioned to `DamageState::Destroyed` this tick —
    /// PathGrid needs rebuild so A* sees collapsed cells as non-traversable
    /// starting next tick. Matches gamemd's one-tick-delayed visibility.
    pub bridge_state_changed: bool,
    pub movement: movement::MovementTickStats,
}
```

**Step 2: Add the field to the `TickResult` construction site.**

In `src/sim/world/mod.rs` `advance_tick`, find the `TickResult { ... }` literal at lines 1485-1493 and add the new field. Initialize with a placeholder `false` for now — Task 3 wires the real value:

```rust
        self.tick = execute_tick;
        let state_hash = self.state_hash();
        TickResult {
            tick: self.tick,
            executed_commands,
            state_hash,
            spawned_entities,
            destroyed_structure,
            ownership_changed: passenger_ownership_changed,
            bridge_state_changed: false, // wired in Task 3
            movement: movement_stats,
        }
```

**Step 3: Verify the field compiles cleanly.**

Run: `cargo check --lib`

Expected: PASS. Any `TickResult { ... }` construction site that doesn't include the new field will fail with E0063 — fix each by adding `bridge_state_changed: false`. (At the time of writing, `world/mod.rs:1485` is the only construction site.)

**Step 4: Commit.**

```
sim/world: add bridge_state_changed flag to TickResult

Plumbing for the bridge-collapse PathGrid refresh — signal that
BridgeRuntimeState mutated this tick so the app can rebuild
state.path_grid before the next advance_tick. Placeholder value
wired through orchestrator return in the next commit.
```

---

### Task 2: Change `apply_bridge_damage_events` return type to `bool`

**Why:** Makes the orchestrator's collapse signal observable to the world tick. The existing `Vec<u64>` return is documented as "do not use" and only bound to discard.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs:51-153` (`apply_bridge_damage_events` body and docstring)
- Modify: `src/sim/world/world_tests.rs:523, 576` (test bindings)

**Pattern:** Simple return-type change; bool semantics mirror `combat_result.structure_destroyed`.

**Step 1: Update the function signature and docstring.**

In `src/sim/world/bridge_orchestrator.rs`, replace the docstring + signature (currently around lines 28-55) with:

```rust
/// Drain a batch of `BridgeDamageEvent`s through the 4-path dispatcher.
///
/// Per-event behavior:
/// 1. Outer gate: if `SpecialFlags::DestroyableBridges` is clear, bail
///    early — bridges are immune.
/// 2. For each event, evaluate paths in fixed order
///    `HighSM → LowSM → LowDirect → HighDirect`.
/// 3. For each matching path, run the per-path RNG gate against
///    BridgeStrength (`damage > rand(1..=BridgeStrength)`). IonCannon
///    bypasses the gate.
/// 4. State-machine paths get up to 3 retries when the warhead is
///    IonCannon (4 attempts total). Direct-overlay paths are single-shot.
/// 5. The first path that produces a non-`NoChange` outcome is the
///    winner; subsequent paths skip for that event.
///
/// Returns `true` if any event in the batch produced a `StateOutcome::Collapsed`
/// — i.e. at least one bridge cell transitioned to `DamageState::Destroyed`.
/// Callers use this to signal `TickResult.bridge_state_changed` so the app
/// rebuilds the PathGrid before next tick's movement runs.
///
/// Cascade side-effects (kill / DropIn / debris / rim / zone) run unconditionally
/// when matching outcomes are present in this batch — they don't depend on
/// the return value.
pub(crate) fn apply_bridge_damage_events(
    sim: &mut Simulation,
    rules: &RuleSet,
    events: &[BridgeDamageEvent],
) -> bool {
```

**Step 2: Update the body — drop `despawned_ids`, return bool.**

In the same function (lines 56-153), make these edits:

Remove the `despawned_ids` declaration at line 56:
```rust
    let despawned_ids: Vec<u64> = Vec::new();
    if events.is_empty() {
        return despawned_ids;
    }
```
Replace with:
```rust
    if events.is_empty() {
        return false;
    }
```

Replace the early returns inside `bridge_strength` resolution (lines 62-65):
```rust
    let bridge_strength = match sim.bridge_state.as_ref() {
        Some(bs) if bs.is_destroyable() => bs.bridge_strength(),
        _ => return despawned_ids,
    };
```
With:
```rust
    let bridge_strength = match sim.bridge_state.as_ref() {
        Some(bs) if bs.is_destroyable() => bs.bridge_strength(),
        _ => return false,
    };
```

At the end of the function (line 152), replace:
```rust
    despawned_ids
}
```
With:
```rust
    // state_changed = "at least one cell collapsed this batch". The destroyed_set
    // is built from StateOutcome::Collapsed outcomes earlier in this function;
    // if it's non-empty, real work happened.
    !destroyed_set.is_empty()
}
```

Note: `destroyed_set` is already declared earlier in the function (line 74) and populated from `StateOutcome::Collapsed` outcomes. We're reading it after the cascade work is done.

**Step 3: Update the world tick caller.**

In `src/sim/world/mod.rs` find the call at line 1241:
```rust
            let _bridge_fallout_ids =
                crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                    self,
                    rules,
                    &combat_result.bridge_damage_events,
                );
```

Replace with (note: bound to `bridge_state_changed` local — Task 3 wires it into `TickResult`):
```rust
            let bridge_state_changed =
                crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                    self,
                    rules,
                    &combat_result.bridge_damage_events,
                );
            let _ = bridge_state_changed; // wired into TickResult in Task 3
```

**Step 4: Update tests that bind the return.**

In `src/sim/world/world_tests.rs`, rename all 6 sites that bind `_despawned` to the orchestrator's return. Use `Edit` with `replace_all: true`:

- `old_string`: `let _despawned = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events`
- `new_string`: `let _state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events`
- `replace_all`: true

This covers sites at lines 523, 576, 629, 690, 743, and 799. The other 7 call sites in this file (lines 872, 908, 1084, 1132, 1172, 1210, 1287) already use `let _ = ...` and need no rename — discarding a bool is identical to discarding a `Vec<u64>` at the call site.

The semantics change (Vec<u64> → bool) but the test bodies don't read the value — they query `sim.bridge_state` afterward.

**Step 5: Verify compile + tests.**

Run: `cargo test --lib -p ra2_rust_game sim::world::bridge_orchestrator`

Expected: all orchestrator tests still pass. The return-type change is invisible to existing assertions.

Run: `cargo check --lib`

Expected: PASS.

**Step 6: Commit.**

```
sim/world: apply_bridge_damage_events returns bool state_changed

Change the orchestrator's return from Vec<u64> (documented "do not
use") to a bool indicating "any cell collapsed this batch". Enables
wiring the PathGrid refresh trigger in the next commit. Existing
callers and tests bound the return to discard; the cascade side
effects are unchanged.
```

---

### Task 3: Wire orchestrator return into `TickResult.bridge_state_changed`

**Why:** Connects the sim-side signal to the cross-boundary `TickResult` so the app can observe it.

**Files:**
- Modify: `src/sim/world/mod.rs:1241-1246` (drop the temporary `_` binding) and `1485-1493` (replace the placeholder)

**Pattern:** Mirror how `combat_result.structure_destroyed` flows into `destroyed_structure |= combat_result.structure_destroyed;` (`world/mod.rs:1228`).

**Step 1: Move the local binding to the outer scope of `advance_tick`.**

In `src/sim/world/mod.rs`, around line 997 (just after `let mut destroyed_structure = false;`), add:
```rust
        let mut bridge_state_changed = false;
```

**Step 2: Wire the orchestrator return into the local.**

In `src/sim/world/mod.rs:1241-1246`, replace the temporary binding from Task 2:
```rust
            let bridge_state_changed_this_call =
                crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                    self,
                    rules,
                    &combat_result.bridge_damage_events,
                );
            let _ = bridge_state_changed_this_call; // wired into TickResult in Task 3
```

With:
```rust
            bridge_state_changed |=
                crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                    self,
                    rules,
                    &combat_result.bridge_damage_events,
                );
```

**Step 3: Wire the local into `TickResult`.**

In `src/sim/world/mod.rs:1485-1493`, replace the placeholder from Task 1:
```rust
            bridge_state_changed: false, // wired in Task 3
```

With:
```rust
            bridge_state_changed,
```

**Step 4: Verify.**

Run: `cargo check --lib`

Expected: PASS.

Run: `cargo test --lib sim::world::tests::test_bridge_damage_rebuilds_path_grid`

Expected: PASS. The existing test doesn't check `tick_result.bridge_state_changed` yet (Task 6 adds that assertion); this run just confirms nothing regressed.

**Step 5: Commit.**

```
sim/world: propagate bridge collapse signal into TickResult

Wire apply_bridge_damage_events' bool return through advance_tick
into TickResult.bridge_state_changed. App-side consumer lands next.
```

---

### Task 4: Replace `path_grid_base.clone()` with fresh `from_resolved_terrain_with_bridges` build

**Why:** This is the core fix. The static `path_grid_base` cache is the bug; replacing the clone with a fresh build from `terrain + sim.bridge_state` makes `state.path_grid` reflect runtime bridge state. The signal wiring from Tasks 1-3 means this fires at the right time without needing further plumbing.

**Files:**
- Modify: `src/app_sim_tick.rs:701-759` (`rebuild_dynamic_path_grid`)

**Pattern:** This function already takes `&mut AppState` and reads both `state.path_grid_base` and `state.simulation`. The new code drops the base read in favor of reading `state.simulation.bridge_state` and `state.simulation.resolved_terrain`.

**Step 1: Rewrite `rebuild_dynamic_path_grid` to build from terrain+bridge_state.**

In `src/app_sim_tick.rs`, replace the entire function body (lines 701-759):

```rust
pub(crate) fn rebuild_dynamic_path_grid(state: &mut AppState) {
    // Build fresh from resolved terrain + current bridge runtime state. We
    // no longer clone state.path_grid_base — it's a load-time cache that
    // doesn't track runtime bridge collapse/repair. PathGrid::from_
    // resolved_terrain_with_bridges queries BridgeRuntimeState.is_bridge_
    // walkable per cell, which is the single source of truth for runtime
    // bridge walkability (and reverts ground walkability under destroyed
    // decks). Stamping building footprints and walls on top of that gives
    // a grid that reflects every runtime mutation (collapse, build, sell,
    // wall placement, wall destruction).
    let Some(rules) = state.rules.as_ref() else {
        return;
    };
    let Some(ref sim) = state.simulation else {
        return;
    };
    let Some(terrain) = sim.resolved_terrain.as_ref() else {
        return;
    };

    let mut grid: PathGrid =
        PathGrid::from_resolved_terrain_with_bridges(terrain, sim.bridge_state.as_ref());

    let mut structures: Vec<(u16, u16, String)> = sim
        .entities
        .values()
        .filter_map(|entity| {
            (entity.category == EntityCategory::Structure).then_some((
                entity.position.rx,
                entity.position.ry,
                sim.interner.resolve(entity.type_ref).to_string(),
            ))
        })
        .collect();
    structures.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    for (rx, ry, type_id) in &structures {
        let obj = rules.object(type_id);
        let foundation = obj.map(|o| o.foundation.as_str()).unwrap_or("1x1");
        let add_occupy: &[(i16, i16)] = obj.map(|o| o.add_occupy.as_slice()).unwrap_or(&[]);
        let remove_occupy: &[(i16, i16)] = obj.map(|o| o.remove_occupy.as_slice()).unwrap_or(&[]);
        grid.block_building_footprint(*rx, *ry, foundation, add_occupy, remove_occupy);
    }

    // Block wall overlay cells (auto-filled walls have no entity but still block movement).
    if let Some(registry) = &state.overlay_registry {
        for entry in &state.overlays {
            let is_wall = registry
                .flags(entry.overlay_id)
                .map(|f| f.wall)
                .unwrap_or(false);
            if is_wall {
                grid.block_building_footprint(entry.rx, entry.ry, "1x1", &[], &[]);
            }
        }
    }

    state.path_grid = Some(grid);

    // Rebuild zone connectivity map for instant unreachability detection.
    // The unified PathGrid already contains building/wall/bridge data from
    // resolved terrain, so no separate sync step is needed.
    if let Some(ref mut sim) = state.simulation {
        if let Some(ref grid) = state.path_grid {
            sim.rebuild_zone_grid(grid);
        }
    }
}
```

Key differences vs the original:
- Drop `state.path_grid_base.as_ref()` read.
- Pull `sim.resolved_terrain.as_ref()` and `sim.bridge_state.as_ref()`.
- Build `grid` via `from_resolved_terrain_with_bridges(terrain, sim.bridge_state.as_ref())`.
- Everything downstream (structure stamping, wall stamping, `state.path_grid = Some(grid)`, zone rebuild) is unchanged.

**Step 2: Verify the wall-stamp order assumption.**

Search for any test that exercises wall + bridge interactions:

Run: `cargo test --lib wall.*bridge --tests` (zero hits is OK — we're just confirming no existing test breaks).

Expected: any pre-existing tests pass. Wall stamping after the fresh base build is idempotent on cells that are already blocked, so no behavior change.

**Step 3: Verify `from_resolved_terrain_with_bridges` is reachable from `app_sim_tick.rs`.**

The function is `pub` at [`core.rs:976`](../../src/sim/pathfinding/core.rs#L976). `app_sim_tick.rs` already imports `PathGrid` from `crate::sim::pathfinding` ([`app_sim_tick.rs:701`](../../src/app_sim_tick.rs#L701) uses `PathGrid` in the return shape — already in scope).

Run: `cargo check --lib`

Expected: PASS. If `from_resolved_terrain_with_bridges` isn't directly callable, add explicit import:
```rust
use crate::sim::pathfinding::PathGrid;
```
(or fully-qualify the call; the snippet above uses the bare name, assuming the existing import covers it).

**Step 4: Run the existing bridge tests to confirm no regression.**

Run: `cargo test --lib sim::world::tests::test_bridge_damage_rebuilds_path_grid`

Expected: PASS. This test builds a `PathGrid` directly via `from_resolved_terrain_with_bridges` after damage and asserts non-walkability — it's not affected by `rebuild_dynamic_path_grid` directly. We want to confirm we haven't regressed the function under test.

Run: `cargo test --lib`

Expected: full library test suite passes.

**Step 5: Commit.**

```
app/sim: rebuild PathGrid from terrain+bridge_state, not stale cache

state.path_grid_base was built once at map load from
PathGrid::from_resolved_terrain (no bridges argument) and never
refreshed. After bridge collapse, BridgeRuntimeState was correctly
mutated but the app's PathGrid still reflected intact bridges
forever, so A* would happily route units onto destroyed cells.

Replace the path_grid_base.clone() with a fresh build via
PathGrid::from_resolved_terrain_with_bridges(terrain, bridge_state).
That function already projects bridge runtime state into per-cell
walkability per BRIDGE_DEFERRED_MECHANICS report §3.4 — including
adjacent bridgehead transition flag clears. Structure and wall
stamping on top is unchanged.

path_grid_base field stays populated for now (cleanup is a separate
follow-up).
```

---

### Task 5: Trigger `refresh_after_tick` on `bridge_state_changed`

**Why:** Without this, the rebuild from Task 4 never fires on collapse-only ticks (ticks where no structure was destroyed and no overlay flipped).

**Files:**
- Modify: `src/app_sim_tick.rs:506-513` (the block that sets `refresh_after_tick` based on `TickResult` flags)

**Pattern:** Mirror the existing `destroyed_structure` / `ownership_changed` / `spawned_entities` triggers.

**Step 1: Add the new trigger.**

In `src/app_sim_tick.rs`, find lines 506-513:

```rust
            if tick_result.destroyed_structure {
                refresh_after_tick = true;
            }
            if tick_result.ownership_changed {
                refresh_after_tick = true;
            }
            if tick_result.spawned_entities {
                refresh_after_tick = true;
```

Add immediately after the `destroyed_structure` block:

```rust
            if tick_result.destroyed_structure {
                refresh_after_tick = true;
            }
            if tick_result.bridge_state_changed {
                refresh_after_tick = true;
            }
            if tick_result.ownership_changed {
                refresh_after_tick = true;
            }
```

**Step 2: Verify.**

Run: `cargo check --lib`

Expected: PASS.

Run: `cargo test --lib`

Expected: full library test suite passes. Existing tests don't exercise this app-side path, but the regression run confirms no regressions.

**Step 3: Commit.**

```
app/sim: refresh path grid on bridge collapse tick

When TickResult.bridge_state_changed is set (orchestrator collapsed at
least one cell), trigger refresh_after_tick so rebuild_dynamic_path_grid
runs post-tick. Combined with the Task-4 base-rebuild change, next
tick's A* sees the destroyed cells as non-traversable — matching
gamemd's one-tick-delayed visibility (deferred-mechanics report §5).
```

---

### Task 6: Integration test — end-to-end collapse + refresh

**Why:** Proves the full signal path works: orchestrator → TickResult flag → app rebuild → A* sees blocked cells.

This is the parity-critical verification for ledger items #1, #4, #9 (one-tick delay, ground revert, layer separation).

**Files:**
- Create: `tests/bridge_pathgrid_refresh.rs` (new integration test file)

**Pattern:** Existing integration test pattern lives in `tests/` directory; sim-level pieces live in `world_tests.rs` but the full app-side flow (`rebuild_dynamic_path_grid`) needs `AppState`. If `AppState` is hard to construct in `tests/`, fall back to a sim-level test in `world_tests.rs` that drives `advance_tick`, then directly calls `PathGrid::from_resolved_terrain_with_bridges` to assert the post-tick view matches what the app would build.

**Step 1: Decide the test surface.**

Check whether `AppState` is constructible from `tests/`:

Run: `grep -rn "fn test.*AppState::new" src/ tests/` (or similar).

If `AppState` has no minimal test constructor, write the test at sim level in `src/sim/world/world_tests.rs` as a new function. Driving the actual `rebuild_dynamic_path_grid` requires `AppState`; without it, we test:
- `tick_result.bridge_state_changed` flips to `true` on a collapse tick.
- A freshly-built `PathGrid::from_resolved_terrain_with_bridges` (what the app would build) returns non-walkable for all collapsed cells.

These two assertions together prove the contract — the actual `rebuild_dynamic_path_grid` call is mechanical glue (Task 5).

**Step 2: Write the test at sim level (recommended path).**

In `src/sim/world/world_tests.rs`, add this new test (after `test_bridge_damage_rebuilds_path_grid` at line ~547):

```rust
/// End-to-end: a bridge collapse driven through combat's BridgeDamageEvent
/// pipeline must set `TickResult.bridge_state_changed = true`, AND the
/// PathGrid that the app would rebuild post-tick must show the collapsed
/// cells as non-walkable on the bridge layer.
///
/// Ledger #1 (one-tick delay), #4 (ground revert), #9 (layer separation).
#[test]
fn test_bridge_collapse_signals_pathgrid_refresh() {
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::movement::locomotor::MovementLayer;

    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);

    // Emit a bridge damage event directly via the orchestrator (combat
    // would normally produce these in CombatResult.bridge_damage_events;
    // calling the orchestrator directly is the same code path used inside
    // advance_tick).
    let state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 2,
            ry: 0,
            damage: 20,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    assert!(
        state_changed,
        "orchestrator must signal state_changed=true on collapse"
    );

    // The PathGrid the app would build after this tick (via rebuild_
    // dynamic_path_grid → PathGrid::from_resolved_terrain_with_bridges):
    let post_tick_grid = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );

    for x in 1..=3 {
        assert!(
            !post_tick_grid.is_walkable_on_layer(x, 0, MovementLayer::Bridge),
            "cell ({x}, 0) must not be walkable on bridge layer after collapse"
        );
    }
}

/// No-collapse tick must NOT signal state_changed. Damage events that
/// don't actually collapse a cell (or empty event lists) should leave the
/// path grid untouched.
#[test]
fn test_no_collapse_does_not_signal_refresh() {
    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bridge_state);

    let rules = combat_test_rules();

    // Empty events list: state_changed must be false.
    let state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[],
    );
    assert!(
        !state_changed,
        "empty events must not signal state_changed"
    );
}
```

**Step 3: Verify.**

Run: `cargo test --lib sim::world::tests::test_bridge_collapse_signals_pathgrid_refresh`

Expected: PASS.

Run: `cargo test --lib sim::world::tests::test_no_collapse_does_not_signal_refresh`

Expected: PASS.

Run: `cargo test --lib`

Expected: full library suite passes.

**Step 4: Commit.**

```
sim/world: integration tests for bridge collapse → pathgrid refresh

Two new tests cover the end-to-end contract:
1. test_bridge_collapse_signals_pathgrid_refresh — collapse via the
   orchestrator sets state_changed=true AND the rebuild path produces
   a grid where all collapsed cells are non-walkable on bridge layer.
2. test_no_collapse_does_not_signal_refresh — empty event list
   returns state_changed=false (don't fire unnecessary rebuilds).

Covers ledger items #1 (one-tick delay), #4 (ground revert), #9
(layer separation).
```

---

### Task 7: Regression test — bridge-adjacent cell `transition` flag

**Why:** Ledger #2 / #3 / parity-critical. When the body span collapses, the bridgehead's `transition` flag (bit 0x200 equivalent) must clear, otherwise A* still permits Ground→Bridge entry into the dead span. The design rebuilds the whole grid, so this should work automatically — but the test guards against future per-cell-delta optimizations regressing it.

**Files:**
- Modify: `src/sim/world/world_tests.rs` (append after Task 6's tests)

**Pattern:** Same harness as Task 6.

**Step 1: Add the regression test.**

In `src/sim/world/world_tests.rs`, append:

```rust
/// Regression for ledger #2 / #3: when a bridge body span collapses, the
/// adjacent bridgehead cell must lose its `transition` flag. Otherwise A*
/// would still permit Ground→Bridge entry into the destroyed span.
///
/// This test guards against future per-cell-delta optimizations that might
/// only update the directly-destroyed cell and miss the adjacent
/// bridgehead. The current design rebuilds the whole grid, so the property
/// holds automatically — but if someone introduces incremental rebuilds,
/// this assertion catches the regression.
#[test]
fn test_bridge_collapse_clears_adjacent_bridgehead_transition() {
    use crate::sim::pathfinding::PathGrid;

    let mut sim = Simulation::new();
    let (resolved, bridge_state) = ew_high_bridge_strip_for_dispatch(2, 0, 2, false, 0);
    sim.resolved_terrain = Some(resolved.clone());
    sim.bridge_state = Some(bridge_state);

    // Before damage, build the grid and snapshot transition flags of any
    // bridgehead cells along the strip.
    let grid_before =
        PathGrid::from_resolved_terrain_with_bridges(&resolved, sim.bridge_state.as_ref());
    let bridgeheads_before: Vec<(u16, u16, bool)> = (0..resolved.width())
        .flat_map(|x| (0..resolved.height()).map(move |y| (x, y)))
        .filter_map(|(x, y)| {
            let cell = grid_before.cell(x, y)?;
            cell.transition.then_some((x, y, true))
        })
        .collect();
    assert!(
        !bridgeheads_before.is_empty(),
        "test fixture must have at least one bridgehead cell"
    );

    // Damage event at the middle of the strip — collapses the whole span.
    let mut rules = combat_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    let _ = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
        &mut sim,
        &rules,
        &[BridgeDamageEvent {
            rx: 2,
            ry: 0,
            damage: 20,
            warhead_ref: crate::sim::intern::InternedId::default(),
            is_ion_cannon: true,
            impact_z: 4,
        }],
    );

    // After collapse, the rebuilt grid must show transition=false on every
    // cell that was a bridgehead before. (The body collapse cascades through
    // refresh_endpoint_active_flags and the bridgehead's adjacent body cells
    // are all Destroyed, so is_bridge_walkable returns false for them,
    // which gates the transition flag in from_resolved_terrain_with_bridges.)
    let grid_after = PathGrid::from_resolved_terrain_with_bridges(
        sim.resolved_terrain.as_ref().unwrap(),
        sim.bridge_state.as_ref(),
    );
    for (x, y, _) in &bridgeheads_before {
        let cell = grid_after.cell(*x, *y).expect("cell exists");
        assert!(
            !cell.transition,
            "bridgehead ({x}, {y}) must lose transition flag after adjacent body collapse"
        );
    }
}
```

**Step 2: Verify.**

Run: `cargo test --lib sim::world::tests::test_bridge_collapse_clears_adjacent_bridgehead_transition`

Expected: PASS.

Note: If the fixture `ew_high_bridge_strip_for_dispatch` doesn't actually create bridgehead-flagged cells (the assert "must have at least one bridgehead" fires), check the fixture's setup. The test then needs to be adjusted to use a fixture that does include a bridgehead. Look at how the bridge_orchestrator's existing `rim_refresh_clears_dangling_stub_cells` test sets up adjacency.

**Step 3: Commit.**

```
sim/world: regression test for bridgehead transition clear on collapse

Guards ledger items #2 and #3: bridgehead cells adjacent to a
destroyed body span must lose their transition flag, otherwise A*
would route units through a Ground→Bridge transition onto the dead
span. The current design rebuilds the whole grid so the property
holds automatically; the test catches future incremental-rebuild
regressions.
```

---

### Task 8: Full regression run + clippy

**Why:** Catches subtle interactions the focused tests miss (e.g. structure-stamp ordering after the base swap, wall regressions, zone rebuild double-count perf).

**Files:** None.

**Step 1: Full test run.**

Run: `cargo test`

Expected: full workspace test suite passes — both library tests and any integration tests in `tests/`.

**Step 2: Clippy.**

Run: `cargo clippy --all-targets --no-deps -- -D warnings`

Expected: no new warnings. The changes are small (one function rewrite, one struct field add, one return-type change) — clippy regressions would be surprising.

**Step 3: Verify dead-code warnings on `path_grid_base`.**

Run: `cargo check --lib 2>&1 | grep -i "path_grid_base"`

Expected: zero or one "field is never read" hint. If multiple come up, the cleanup follow-up is bigger than expected — note this in a TODO comment near the field, but do NOT clean up `path_grid_base` in this plan (out of scope per design doc).

If a dead-code warning appears and breaks the build (CI may treat warnings as errors), add `#[allow(dead_code)]` to the `path_grid_base` field with a comment pointing to the cleanup follow-up:

In `src/app.rs` near line 177:
```rust
/// Cached load-time PathGrid base — dead since the 2026-05-12 bridge
/// runtime refresh switched `rebuild_dynamic_path_grid` to build fresh
/// from `from_resolved_terrain_with_bridges`. Field stays for now;
/// follow-up cleanup removes it across app, app_init, app_transitions.
#[allow(dead_code)]
pub(crate) path_grid_base: Option<PathGrid>,
```

Same in `src/app_init.rs:108` and `src/app_transitions.rs:81` if needed.

**Step 4: Commit (only if dead-code annotations were needed).**

```
app: silence dead-code warning on path_grid_base

Field is no longer read after the bridge PathGrid runtime refresh
landed; cleanup is tracked as a follow-up.
```

If no annotations were needed, skip the commit.

---

### Task 9: Manual verification against gamemd.exe

**Why:** Per CLAUDE.md, parity claims need evidence beyond unit-test passes.

**Files:** None.

**Step 1: Behavioral check (manual, in-game).**

Set up a skirmish on a map with a destructible bridge (any standard YR map with HBR/LBR overlays). Send an attacking unit to destroy the bridge. Send a friendly unit toward the now-destroyed bridge.

Expected:
- The collapse animation/debris plays this tick (Phase 5 cascade).
- On the immediately following tick, the friendly unit recomputes its path to route around the bridge.
- The unit does NOT walk onto a destroyed cell. If you watch carefully on the first post-collapse tick, the unit's current path may still cross the bridge (it was computed before collapse), but A* on the next path query refuses to enter the destroyed cells.

Run a side-by-side with gamemd.exe on the same map and the same sequence. The behavior should be indistinguishable.

**Step 2: Adjacent-cell check.**

In the same scenario, after collapse, command a unit to path from ground onto the bridgehead cell adjacent to the destroyed span. Expected: A* refuses (the bridgehead's `transition` flag is now cleared by the rebuild). Compare to gamemd.

**Step 3: No-collapse stability.**

In a long match without bridge damage, verify there's no perceptible perf regression. The rebuild only fires when `refresh_after_tick` triggers, and the new rebuild is the same cost as the old clone-plus-stamp (one full-map walk either way).

**Step 4: Document findings.**

If any divergence is observed against gamemd, note it in `ra2-rust-game-docs/AUDIT_LOG.md` and open a follow-up. If parity holds, no doc change needed.

**Step 5: No commit unless docs change.**

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-12-bridge-pathgrid-runtime-refresh-design.md](2026-05-12-bridge-pathgrid-runtime-refresh-design.md)
- **Ghidra reports:**
  - [`ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md) — §3.4 (bit-write map), §3.5 (caller graph), §5 (tick ordering), §6 (Rust status), §11 (recommendations)
  - `ra2-rust-game-docs/BRIDGE_SYSTEM.md` §SetBridgeDirection
  - `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- **gamemd.exe addresses (kept here, not in Rust source):**
  - `0x47E040` — `CellClass__SetBridgeDirection_NESW`
  - `0x47E470` — `CellClass__SetBridgeDirection_NWSE` (instruction-twin)
  - `0x47DD70` — `CellClass__BlowUpBridge`
  - `0x576BA0` — `ProcessBridgeDamageStateMachine_High`
  - `0x571490` — `ProcessBridgeDamageStateMachine_Low`
- **Related code (current state, line numbers may drift):**
  - [`src/sim/world/mod.rs:73-86, 1241-1246, 1485-1493`](../../src/sim/world/mod.rs)
  - [`src/sim/world/bridge_orchestrator.rs:28-153, 309-324`](../../src/sim/world/bridge_orchestrator.rs)
  - [`src/sim/pathfinding/core.rs:976-1025`](../../src/sim/pathfinding/core.rs)
  - [`src/sim/bridge_state/mod.rs:732, 1131-1147`](../../src/sim/bridge_state/mod.rs)
  - [`src/app_sim_tick.rs:506-513, 629-630, 701-759`](../../src/app_sim_tick.rs)
  - [`src/sim/world/world_tests.rs:504-547`](../../src/sim/world/world_tests.rs) (existing `test_bridge_damage_rebuilds_path_grid`)
- **Known follow-ups (out of scope for this plan):**
  - BridgeRepairHut state machine — `refresh_endpoint_active_flags` needs a bidirectional sibling once repair lands.
  - `path_grid_base` field cleanup — remove field + `app_init`/`app_transitions` plumbing in a separate pass.
