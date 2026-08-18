# Wall Damage & Connection Cleanup Design

## Goal

Wire combat damage events to the existing `damage_wall_overlay()` pipeline and
add binary-faithful neighbor connectivity recompute (with safety-net destruction)
so walls become destructible and visually collapse when isolated.

## Architecture Context

Walls in this codebase have a dual representation:

- **GameEntity** (per-cell wall instance) — owns the cell, carries owner info,
  participates in selection/build legality. Currently has health field that we
  will deprecate as a damage source (per Q2 → Approach (a)).
- **OverlayCell** ([src/sim/overlay_grid.rs:14-26](../../src/sim/overlay_grid.rs#L14))
  — `overlay_data: u8` packs `(damage_stage << 4) | connectivity_bitmask`,
  matching `cell+0x11E` in gamemd.exe.

Connectivity logic ([src/map/overlay.rs:181-275](../../src/map/overlay.rs#L181))
already computes the 4-bit cardinal nibble (N=1, E=2, S=4, W=8) at map-load and
on placement. Damage logic ([src/sim/overlay_grid.rs:239-331](../../src/sim/overlay_grid.rs#L239))
already implements the binary's `CellClass::DestroyOverlay (0x480CB0)` —
probabilistic gate, +0x10 stage increment, recursive same-type chain at
penultimate stage, overlay clear at max-damage-and-isolated.

What is missing:

1. **Combat → wall** plumbing: combat collects `BridgeDamageEvent` when warhead
   has `Wall=yes`, but never calls `damage_wall_overlay()`.
2. **Per-cell connectivity recompute** after damage/destroy: existing
   `compute_wall_connectivity()` is a full O(N) sweep; the binary uses
   `CellClass::PostDestructionWallCleanup (0x480630)` to refresh just the 4
   cardinal neighbors of a destroyed wall.
3. **Auto-destruct safety net** during cleanup: per-overlay-type byte-value
   thresholds that destroy a neighbor wall whose damage stage hit max during
   neighbor refresh.
4. **Wall entity removal** when the overlay destroys.

Adjacent systems that observe wall state:

- Pathfinding: `OverlayGrid::recalc_overlay_passability` already runs from
  `dirty_cells`. Removing a wall will free up the cell automatically.
- Render: `app_instances/overlays.rs` reads `overlay_data` lower nibble for
  frame selection. Already consumes our changes.
- Ore growth: uses on-demand `count_ore_neighbors()`, no stored counter.
  Wall removal automatically permits ore spread without explicit notification.

## Impact Analysis

**Files modified:**
- [src/sim/combat/mod.rs:561-572 + 1186-1215](../../src/sim/combat/mod.rs#L561) —
  emit `WallDamageEvent` alongside (or instead of) the existing
  `BridgeDamageEvent` collection. Wall warheads should produce wall events for
  cells with wall overlays and bridge events for cells with bridge overlays;
  the two are mutually exclusive (a cell either has a wall overlay or a bridge,
  not both).
- [src/app_sim_tick.rs](../../src/app_sim_tick.rs) — new dispatcher loop that
  consumes `WallDamageEvent`s, calls `damage_wall_overlay()`, runs neighbor
  cleanup, and removes destroyed entities.
- [src/sim/overlay_grid.rs:239-331](../../src/sim/overlay_grid.rs#L239) — new
  helpers `recompute_wall_connectivity_at()` and `cleanup_wall_neighbors()`.
- World/entity layer — function to remove a wall entity at a given cell.

**Risk areas:**
- **Determinism:** wall events processed in deterministic order; RNG for the
  probabilistic gate must use sim RNG. Verify `damage_wall_overlay()` doesn't
  call `rand::thread_rng()` anywhere.
- **Mid-tick entity removal:** collect destroyed cell coords during dispatch,
  remove entities at end of dispatch (not during iteration).
- **Recursive cleanup:** safety-net destruction on a neighbor can newly-isolate
  another wall, requiring further cleanup. Use a worklist with a visited set
  to bound iteration.
- **Replay/snapshot:** `OverlayCell` and entities both already serialize.
  No new state is introduced. Should be transparent.

**No breakage expected for:**
- AI threat/target assessment (verified: no reads of wall entity HP).
- UI health bars on walls (would have been decorative anyway with no real damage).
- Render: existing `dirty_cells` mechanism handles repaint.

## Chosen Approach

**Approach B** from brainstorm: per-cell connectivity recompute with
safety-net destruction. Mirrors binary's `PostDestructionWallCleanup` directly.

Approach A (full-pass connectivity recompute) was rejected because it skips
the auto-destruct safety net (ledger items 14a-14f) — fully-damaged walls
that were "held up" by a connection would never collapse when isolated by a
neighbor's destruction, breaking the characteristic visual cascade.

Approach C (dedicated `src/sim/walls/` module) was rejected as YAGNI — the
total wall-specific logic is ~150 lines, doesn't justify a separate module.

## Tiny-Detail Ledger

Carried forward from brainstorm. All in-scope items have a design home; the
two `UNKNOWN` items have been resolved.

| # | Detail | Source | Design home |
|---|--------|--------|-------------|
| 1 | Probabilistic gate: `damage < Strength && RandomRanged(0, Strength) > damage → no-op` | doc §4.1 | existing `damage_wall_overlay()` |
| 2 | `damage == sentinel` (forced destroy) bypasses gate | doc §4.1 | existing |
| 3 | After gate: increment stage by `+0x10` always | doc §4.1 | existing |
| 4 | Chain triggers iff `new_stage == DamageLevels - 1 && DamageLevels > 2` | doc §4.1 | existing |
| 5 | Chain payload: 200 damage to cardinal same-type pristine neighbors | doc §4.1 | existing |
| 6 | Cardinal-only (4-way), not 8-way | doc §4.1 | existing |
| 7 | Order: gate → increment → chain → destroy-check | doc §4.1 | existing |
| 8 | Destruction gate: stage at max AND connectivity == 0 (or forced) | doc §4.1 | existing |
| 9 | Mid-segment max-damage walls stay visible until isolated | doc §4.2 | preserved by destruction gate |
| 10 | On destroy: clear overlay_id + overlay_data; remove entity | doc §4.1 | new dispatcher (entity removal) |
| 11 | After destroy: rebuild connectivity nibble on 4 cardinal neighbors | doc §4.1, §5 | new `recompute_wall_connectivity_at()` |
| 12 | Bit assignment N=0, E=1, S=2, W=3 | doc §2.1 | existing (verified matches binary) |
| 13 | Same-type-only matching | doc §3 | new helper (mirrors existing logic) |
| 14a | GASAND (0): byte ∈ {0x10, 0x20} → destroy | doc §5.2 | new helper threshold table |
| 14b | CYCL (1): byte == 0x20 → destroy | doc §5.2 | same |
| 14c | GAWALL (2): byte ∈ {0x20, 0x30} → destroy | doc §5.2 | same |
| 14d | BARB (3): byte == 0x10 → destroy | doc §5.2 | same |
| 14e | overlay 0x16: byte ∈ {0x10, 0x20} → destroy | doc §5.2 | same |
| 14f | NAWALL (0x1A): byte ∈ {0x20, 0x30} → destroy | doc §5.2 | same |
| 15 | Safety-net destruction is recursive (cleanup neighbors of newly-destroyed) | doc §5.1 | new `cleanup_wall_neighbors()` worklist with visited set |
| 16 | RNG must be deterministic sim RNG | CLAUDE.md | dispatcher passes sim RNG; verify existing impl |
| 17 | Wall warhead gate: `WarheadType.wall` | combat/mod.rs:561 | existing |
| 18 | `Strength=` per overlay type from rules.ini | doc §8 | existing |
| 19 | `DamageLevels=` per overlay type from art.ini | doc §8 | existing |
| 20 | Zone classification refreshed via `dirty_cells` | doc §4.1 | existing infrastructure |
| 21 | OreNeighborCount 8-neighbor decrement | doc §4.1 | **N/A** — Rust uses on-demand count |
| 22 | LAT retrigger via RecalcAttributes on 5 cleaned-up cells | doc §6 | **accepted drift** — Rust LAT is computed at map load; walls don't expose Pave LAT issues in normal play |
| 23 | Wildcard 0xF3 overlay handling | doc §3 | **out of scope** |
| 24 | Cross-system fence-post BuildingType connections | doc §5 | **out of scope** |

Two items accepted as drift (#22, OreNeighborCount-related impl differences):
- LAT retrigger: not implemented. If a Pave-LAT-on-wall-destroy issue surfaces
  in playtest, revisit.
- OreNeighborCount: replaced by `count_ore_neighbors()` which is logically
  equivalent but architecturally different. No drift in observable behavior.

## Design

### Components

#### 1. `WallDamageEvent` (combat → world)

```rust
pub struct WallDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
}
```

Sentinel value `damage == u16::MAX` represents forced destroy (matches
existing `damage_wall_overlay()` convention).

Emitted from combat at the same sites as `BridgeDamageEvent`:
- AoE damage path ([combat/mod.rs:1186-1215](../../src/sim/combat/mod.rs#L1186))
- Direct fire path ([combat/mod.rs:561-572](../../src/sim/combat/mod.rs#L561))
- Death weapon (warhead-with-Wall=yes on unit destruction)

Emission rule: when `warhead.wall && damage > 0`, look up the cell's overlay.
- If cell has a bridge overlay → emit `BridgeDamageEvent` (existing behavior).
- If cell has a wall overlay → emit `WallDamageEvent`.
- Both empty → no event.

The two are mutually exclusive in a single cell (binary verified).

#### 2. `recompute_wall_connectivity_at(grid, registry, rx, ry) -> RecomputeResult`

Lives in `src/sim/overlay_grid.rs`. Returns whether the cell was destroyed
by the safety-net check (so caller can chain cleanup).

```rust
pub enum RecomputeResult {
    /// Cell was not a wall, or had no change.
    NoChange,
    /// Connectivity nibble changed but cell remains.
    Updated,
    /// Cell was destroyed by the safety-net threshold.
    Destroyed,
}

pub fn recompute_wall_connectivity_at(
    grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
) -> RecomputeResult;
```

Logic:
1. Read cell's `overlay_id`. If None or not a wall, return `NoChange`.
2. For each cardinal direction (N, E, S, W), inspect the neighbor cell. Set
   the corresponding bit if the neighbor has a wall overlay of the **same
   `overlay_id`** (matches binary's primary same-type branch).
3. Read the current full byte. Compute new connectivity nibble; combine with
   existing damage nibble.
4. If unchanged from current byte → return `NoChange`.
5. Apply per-type safety-net threshold (ledger items 14a-14f):
   - Lookup table mapping `overlay_id → set of "auto-destruct" full bytes`.
   - If new byte is in the set → clear overlay (set `overlay_id = None`,
     `overlay_data = 0`), mark cell dirty, return `Destroyed`.
6. Otherwise write the new byte back, mark cell dirty, return `Updated`.

Threshold table (from doc §5.2):

```rust
fn auto_destruct_threshold(overlay_id: u8, full_byte: u8) -> bool {
    match overlay_id {
        0    => matches!(full_byte, 0x10 | 0x20),  // GASAND
        1    => full_byte == 0x20,                 // CYCL
        2    => matches!(full_byte, 0x20 | 0x30),  // GAWALL
        3    => full_byte == 0x10,                 // BARB
        0x16 => matches!(full_byte, 0x10 | 0x20),
        0x1A => matches!(full_byte, 0x20 | 0x30),  // NAWALL
        _    => false,
    }
}
```

#### 3. `cleanup_wall_neighbors(grid, registry, rx, ry) -> Vec<(u16, u16)>`

Lives in `src/sim/overlay_grid.rs`. Returns the list of cells destroyed by
the cleanup (so caller can remove their entities).

```rust
pub fn cleanup_wall_neighbors(
    grid: &mut OverlayGrid,
    registry: &OverlayTypeRegistry,
    rx: u16,
    ry: u16,
) -> Vec<(u16, u16)>;
```

Logic (worklist with visited set):

```rust
let mut destroyed = Vec::new();
let mut visited = HashSet::new();
let mut worklist = VecDeque::new();
worklist.push_back((rx, ry));

while let Some((cx, cy)) = worklist.pop_front() {
    for (dx, dy) in CARDINAL_4 {
        let nx = cx as i32 + dx;
        let ny = cy as i32 + dy;
        if !grid.in_bounds(nx, ny) { continue; }
        let neighbor = (nx as u16, ny as u16);
        if !visited.insert(neighbor) { continue; }

        match recompute_wall_connectivity_at(grid, registry, neighbor.0, neighbor.1) {
            RecomputeResult::Destroyed => {
                destroyed.push(neighbor);
                worklist.push_back(neighbor);  // its neighbors may now be isolated
            }
            _ => {}
        }
    }
}

destroyed
```

The visited set bounds work to O(cells_visited) and prevents re-processing
the same neighbor.

#### 4. Dispatcher loop in `app_sim_tick.rs`

```rust
let wall_events = std::mem::take(&mut sim.wall_damage_events);
let mut entities_to_remove: Vec<(u16, u16)> = Vec::new();

for event in wall_events {
    let result = damage_wall_overlay(
        &mut sim.overlay_grid,
        &registry,
        &mut sim.rng,
        event.rx, event.ry,
        event.damage,
    );

    for &(dx, dy) in &result.destroyed_cells {
        entities_to_remove.push((dx, dy));
        let chained = cleanup_wall_neighbors(&mut sim.overlay_grid, &registry, dx, dy);
        entities_to_remove.extend(chained);
    }
}

// Deduplicate (chain may cover same cell twice in pathological cases)
entities_to_remove.sort_unstable();
entities_to_remove.dedup();

for (rx, ry) in entities_to_remove {
    remove_wall_entity_at(&mut sim.entities, rx, ry);
}
```

### Interfaces / Contracts

**New public functions:**
- `OverlayGrid::recompute_wall_connectivity_at(registry, rx, ry) -> RecomputeResult`
- `OverlayGrid::cleanup_wall_neighbors(registry, rx, ry) -> Vec<(u16, u16)>`

**New event type:**
- `WallDamageEvent { rx, ry, damage }` — added to combat output stream

**Existing function, additional caller:**
- `damage_wall_overlay()` is now called from the world dispatcher (was
  previously uncalled by combat).

**No public API breakage:** `compute_wall_connectivity()` (full-pass) remains
for map-load and bulk placement. The new per-cell helpers complement it.

### Data Flow

```
                                                    ┌──────────────┐
                                                    │  WarheadType │
                                                    │  Wall = yes  │
                                                    └──────┬───────┘
                                                           │
  ┌──────────┐    fires    ┌────────────┐    .wall == true ↓     ┌─────────────────────┐
  │  Weapon  │────────────▶│  combat::  │─────────────────────────▶│ WallDamageEvent     │
  │  Damage  │             │   tick     │                          │ { rx, ry, damage }  │
  └──────────┘             └────────────┘                          └──────────┬──────────┘
                                                                              │
                                                                              ▼
                                  ┌──────────────────────────────────────────────────────┐
                                  │  app_sim_tick: wall damage dispatcher                │
                                  │  for each event:                                      │
                                  │    1. damage_wall_overlay(grid, registry, rng, …)    │
                                  │       → returns { changed_cells, destroyed_cells }    │
                                  │    2. for each destroyed:                             │
                                  │       a. queue entity removal                         │
                                  │       b. cleanup_wall_neighbors(grid, registry, …)    │
                                  │          → returns chain-destroyed cells              │
                                  │          → queue their entity removals                │
                                  │    3. remove queued entities (dedup'd)                │
                                  └──────────────┬───────────────────────────────────────┘
                                                 │
                                                 ▼
                                          ┌─────────────────┐
                                          │  OverlayGrid    │
                                          │  dirty_cells    │
                                          │  (existing      │
                                          │   mechanism)    │
                                          └────────┬────────┘
                                                   │
                              ┌────────────────────┼─────────────────────┐
                              ▼                    ▼                     ▼
                      ┌─────────────┐   ┌──────────────────┐   ┌────────────────┐
                      │  Render     │   │  Pathfinding     │   │  Zone map      │
                      │  (frame     │   │  (passability    │   │  (zone class   │
                      │   refresh)  │   │   recalc)        │   │   refresh)     │
                      └─────────────┘   └──────────────────┘   └────────────────┘
```

### Error Handling

- **Out-of-bounds cell coords in event:** silently skip (defensive — combat
  shouldn't emit OOB events but a network desync or scripted cheat could).
- **Cell has no overlay:** `damage_wall_overlay()` already no-ops if cell has
  no wall overlay; defensive check only.
- **Entity not found at destroyed cell:** log a warn and continue (shouldn't
  happen — wall entity and overlay are placed together — but a corrupted
  state shouldn't crash the tick).
- **Chain reaction unbounded:** prevented by the `visited` set in
  `cleanup_wall_neighbors`. Worst case O(map_cells) which is bounded.

### Testing Strategy

**Unit tests in `src/sim/overlay_grid.rs`:**
- `recompute_after_neighbor_destroy_updates_nibble`: place 3 walls in a row,
  destroy middle, verify outer two have updated connectivity nibbles.
- `safety_net_destroys_isolated_max_damage_wall`: set up a wall at full
  damage with one connection; destroy that connection; verify the now-isolated
  wall is also destroyed by the safety net.
- `cleanup_chain_terminates`: set up a row of 5 max-damage walls; destroy one
  end; verify the chain unwinds without infinite loop.
- `recompute_no_op_for_non_wall`: call on an ore cell, verify no-op.
- `cleanup_handles_oob_neighbors`: destroy a wall at the map edge, verify no
  panic.

**Integration tests:**
- New test in `src/sim/combat/combat_tests.rs`:
  `wall_warhead_damages_overlay_and_eventually_destroys_it` — fire a
  wall-warhead weapon at a wall many times, verify damage stage progresses
  and eventually overlay clears + entity removed.
- New test:
  `wall_chain_reaction_destroys_concrete_segment` — fire enough damage at one
  cell of a row of GAWALL to push it to max stage, verify cardinal neighbors
  receive 200 chain damage and propagate.

**Determinism tests:**
- `wall_damage_deterministic_across_replays`: run the same wall-damage
  sequence with the same seed twice, verify identical destroyed-cell sets.

## Architectural Decisions

**Patterns followed:**
- Event-collection-then-dispatch (mirrors existing `BridgeDamageEvent` flow).
- `dirty_cells` propagation for downstream system invalidation
  (mirrors existing `OverlayGrid` mutation pattern).
- Sim RNG ownership (no `rand::thread_rng()`).
- Visited-set-bounded recursion (mirrors patterns in pathfinding zone fill).

**Patterns deviated from:** None.

**Tech debt introduced:** None. The code is additive and well-scoped.

**Tech debt addressed:** Walls becoming destructible was an existing implicit
gap; this design closes it without architectural churn.

**Out of scope (deferred):**
- Wildcard `0xF3` overlay handling (likely TS legacy).
- Cross-system fence-post `BuildingType` connections (Firestorm Wall, Laser
  Fence) — needed only when those buildings are implemented.
- FriendlyWall passability (Can_Enter_Cell return code 4) — separate concern,
  not part of this design (was out of scope at brainstorm Q1).
- LAT retrigger on cleanup (item #22) — accepted drift; revisit if Pave-LAT-on-
  wall-destroy issues surface.

## Alternatives Considered

**Approach A (full-pass connectivity recompute on every wall event):**
Rejected because it doesn't apply the per-type auto-destruct safety net
(ledger items 14a-14f). Mid-segment max-damage walls would never collapse
when isolated by neighbor destruction, breaking the binary's characteristic
visual cascade.

**Approach C (dedicated `src/sim/walls/` module):** Rejected as YAGNI. Total
wall-specific logic is ~150 lines; module overhead unjustified.

**Entity-authoritative HP (Q2 option b):** Rejected because the binary has no
entity-style HP for walls. The 4-stage damage nibble + probabilistic gate is
the entire HP system. Translating subtractive HP back to a probabilistic gate
would be inventing logic the binary doesn't have.

**Friendly-wall passability (Q1 option c):** Rejected from this scope.
Allies-drive-through-walls is a separate behavior; without it, allies just
route around like enemies. Not visibly broken, just suboptimal AI.
