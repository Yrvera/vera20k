# ObjectSubstrate Slice 5 — Enter-Order Counter Ownership Design

## Goal

Replace the hand-threaded raw `&mut u64` occupancy enter-order counter with a typed
`EnterOrderCounter` newtype whose only mutator is `next()`, so the increment formula has
exactly one home — a pure-determinism, hash-identical refactor with no `SNAPSHOT_VERSION` bump.

## Architecture Context

The enter-order counter assigns each entity a monotonically increasing "when did I enter a
cell-list" number, stored per-entity as `GameEntity.occupancy_enter_order` (game_entity.rs:218,
default `stable_id` at creation, game_entity.rs:509). `OccupancyGrid::rebuild` reconstructs
cell-list order on load by sorting entities on `(occupancy_enter_order, stable_id)`
(occupancy.rs:121) — so the **per-entity field** reproduces order after a load, and the **global
counter** `next_occupancy_enter_order` survives load only because it is a serialized field on
`ObjectSubstrate` (substrate.rs:31, started at `1` in `new()` substrate.rs:52). Both are hashed:
the counter at world_hash.rs:49, the per-entity field at world_hash.rs:387.

The same read-increment-write triple (`order = counter; counter = counter.saturating_add(1);
field = order`) is copy-pasted at three live sites:

- `Simulation::add_entity_occupancy` (mod.rs:793-795) — spawn / reveal / unload occupancy add;
  has `&mut self`, reads the substrate counter directly.
- `movement_tick.rs:1316-1318` — a movement branch holding a real `&mut entity`.
- `movement_step.rs:1198-1200` — inside `process_cell_crossings`, a deep free function that
  operates on **decomposed `&mut` entity fields** (it holds `&mut OccupancyGrid` plus the two
  `&mut u64`s, but not `&mut entity` / `&mut substrate`). This is why the counter is hand-threaded
  `advance_tick` (mod.rs:1670) → `tick_movement_with_grids` → `movement_tick` (param at
  movement_tick.rs:826) → `process_cell_crossings` (param at movement_step.rs:910).

The legacy single-grid wrapper `tick_movement_with_grid` and its `let mut
next_occupancy_enter_order = 1` (movement/mod.rs:281, plus `movement_tests.rs:1730/1806` and
`prone_speed_tests.rs:84`) are **test-only** — production always uses the plural
`tick_movement_with_grids` with the substrate counter. `movement_tests.rs:761/785` pass
`&mut …substrate.next_occupancy_enter_order` directly.

## Impact Analysis

| Touched | What changes |
|---------|--------------|
| `src/sim/world/substrate.rs` | Define `EnterOrderCounter`; field type `u64` → `EnterOrderCounter`; `new()` init `1` → `EnterOrderCounter::new()` |
| `src/sim/world/mod.rs` | `add_entity_occupancy` triple → `counter.next()`; tick call site passes `&mut EnterOrderCounter` |
| `src/sim/movement/movement_tick.rs` | Param type `&mut u64` → `&mut EnterOrderCounter`; assign-site uses `.next()` |
| `src/sim/movement/movement_step.rs` | `process_cell_crossings` counter param type; assign-site uses `.next()` |
| `src/sim/movement/mod.rs` (test wrappers) | local `= 1` → `= EnterOrderCounter::new()` |
| `src/sim/movement/movement_tests.rs`, `prone_speed_tests.rs` | local counter inits → `EnterOrderCounter::new()` |
| `src/sim/world/world_hash.rs` | **Unchanged** — relies on derived-`Hash` equivalence (see below) |

**Risk areas:** (1) hash drift if the newtype hashes differently than the bare `u64`; (2)
save/load incompatibility if the serialized bytes differ; (3) an accidental change to the
per-crossing increment cadence. All three are guarded — see Tiny-Detail Ledger and Testing.

**Determinism:** the counter and per-entity order are both hashed and serialized; this refactor
must change neither value nor cadence. No tick-ordering change (the assign-site stays exactly
where it is — inside the per-crossing loop).

## Chosen Approach

**Approach B — `EnterOrderCounter` newtype with `next()`, serde-`transparent`, derived `Hash`.**
The counter stays in `ObjectSubstrate` (serialized, hashed at the same path). The raw `&mut u64`
threaded through movement becomes `&mut EnterOrderCounter`; the increment formula lives only in
`next()`. Chosen over:

- **A (shared free helper, counter stays raw `u64`):** de-duplicates the formula but leaves the
  bare `&mut u64` threading and the mis-increment hazard. B gives the same de-duplication plus
  type safety for the same effort.
- **C (full `substrate.move_cell(id, to)` per design §6 literal text):** a movement-architecture
  rewrite — `process_cell_crossings` decomposes the entity into `&mut` fields precisely to avoid
  holding `&mut entity`/`&mut substrate` across the crossing loop; reversing that is large
  blast-radius with high accidental-hash-change risk. Deferred as a movement-layer follow-up, not
  part of this slice.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct EnterOrderCounter(u64);

impl EnterOrderCounter {
    /// Counters start at 1; 0 is the reserved sentinel.
    pub(crate) const fn new() -> Self { Self(1) }
    /// Return the current order value and advance. Saturating — never wraps,
    /// matching the pre-consolidation `saturating_add(1)` at every assign-site.
    pub(crate) fn next(&mut self) -> u64 {
        let order = self.0;
        self.0 = self.0.saturating_add(1);
        order
    }
}
```

The three assign-sites collapse to:
```rust
let order = /* counter */.next();
entity.occupancy_enter_order = order;
```

## Tiny-Detail Ledger

Every item must survive the refactor; the replay-hash + saveload + rebuild suites are the oracles.

- **Increment cadence: once per cell-crossing**, inside the movement loop (movement_step.rs:1198
  is within the `loop` at :925), and once per spawn/reveal occupancy-add. A mover crossing 2 cells
  in one step bumps the counter twice. The assign-site must NOT move out of the per-crossing loop.
  [code: movement_step.rs:1198-1200, movement_tick.rs:1316-1318, mod.rs:793-795]
- **Formula: saturating, never wrapping** — `order = counter; counter = counter.saturating_add(1)`.
  `next()` reproduces this verbatim, including saturation at `u64::MAX`. [code: mod.rs:794,
  movement_tick.rs:1317, movement_step.rs:1199]
- **Order assigned before the grid mutation** — the field write precedes the `add`/`move_entity`
  call. Preserved (call `next()`, write field, then call the grid). [code: movement_step.rs:1200→:1201]
- **Counter starts at 1**, 0 reserved. `EnterOrderCounter::new()` → `Self(1)`. [code: substrate.rs:52]
- **Entity field defaults to `stable_id`** at creation, overwritten by a counter value only on
  first cell-list entry; a never-moving entity keeps `order == stable_id`. Untouched by this slice.
  [code: game_entity.rs:509]
- **Consumer sort key `(occupancy_enter_order, stable_id)`** — rebuild uses only the per-entity
  field, not the global counter. Untouched. [code: occupancy.rs:121]
- **Both counter and per-entity field are hashed + serialized.** The newtype must hash and
  serialize identically to the bare `u64`:
  - *Hash:* `#[derive(Hash)]` on a single-field tuple struct hashes exactly the inner field with
    no discriminant/length prefix → bit-identical to `u64::hash`. world_hash.rs:49 stays unchanged.
    [code: world_hash.rs:49,387]
  - *Serde:* `#[serde(transparent)]` over `u64` emits the same wire bytes for the field → save/load
    compatible, no `SNAPSHOT_VERSION` bump. [code: substrate.rs:22-44 derive(Serialize/Deserialize)]
- **Test wrappers' local counter starts at 1**, independent of the substrate counter; production
  uses the substrate counter. The test inits become `EnterOrderCounter::new()`. [code: movement/mod.rs:281]

## Design

### Components

- `EnterOrderCounter(u64)` — new newtype in `substrate.rs` (or a small sibling), `pub(crate)`.
  Sole mutator `next()`; constructor `new()`.
- `ObjectSubstrate.next_occupancy_enter_order` — type changes `u64` → `EnterOrderCounter`; same
  field name, same serialized position, same hash site.

### Interfaces / Contracts

- **Added:** `EnterOrderCounter::new() -> Self` (const), `EnterOrderCounter::next(&mut self) -> u64`.
- **Changed (type only):** the counter parameter on `tick_movement_with_grids`,
  `movement_tick`'s processing fn, and `process_cell_crossings`: `&mut u64` → `&mut EnterOrderCounter`.
  The **entity-field** parameter `occupancy_enter_order: &mut u64` stays `&mut u64`.
- **Unchanged:** `OccupancyGrid::add` / `move_entity` signatures; `world_hash` call sites;
  serialized layout; `SNAPSHOT_VERSION`.

### Data Flow

`advance_tick` owns `substrate.next_occupancy_enter_order: EnterOrderCounter` → passes
`&mut` into `tick_movement_with_grids` → `movement_tick` → `process_cell_crossings`. At each cell
crossing the function calls `counter.next()`, writes the returned `u64` into the entity's
`occupancy_enter_order` field, then mutates the grid. `add_entity_occupancy` calls
`self.substrate.next_occupancy_enter_order.next()` directly under `&mut self`.

### Error Handling

None — pure value bookkeeping. `next()` saturates rather than panicking at `u64::MAX`.

### Testing Strategy

- **Oracles (must stay green, hash bit-identical):** full lib suite, replay-hash / `world_hash`
  tests, `saveload_*` (esp. `saveload_occupancy_list_order_matches_incremental`), occupancy-rebuild
  test.
- **New focused unit test on `EnterOrderCounter`:** `new()` == 1; `next()` returns the
  pre-increment value then advances (1 → returns 1, now 2 → returns 2); saturation at `u64::MAX`
  (`next()` returns `MAX` and stays `MAX`).

## Architectural Decisions

- **Follows** the project's existing newtype-wrapper pattern for typed sim quantities and the
  Slice 1-4 "one owner for a cross-cutting invariant" funnel approach.
- **Deviates from** the design's literal §6 wording ("`move_cell` owns the counter") because the
  counter must stay in a serialized location (`OccupancyGrid` is `#[serde(skip)]`) and the full
  `move_cell(id,to)` movement rewrite is out of scope for a hash-identical slice. The faithful
  reading is "one typed owner of the increment, no bare `&mut u64`," which B delivers.
- **No tech debt introduced.** The deferred C-style `move_cell` consolidation is recorded as a
  follow-up tied to any future movement-layer rework, not a debt this slice creates.

## Alternatives Considered

- **A — shared free helper, raw `u64`:** smaller but keeps the bare `&mut u64` threading and the
  mis-increment hazard; same de-dup payoff as B without the type safety. Rejected.
- **C — `substrate.move_cell(id, to)`:** the design's literal text; a movement-architecture
  rewrite with high accidental-hash-change risk. Deferred, not part of this slice.
- **Move the counter into `OccupancyGrid`:** rejected — the grid is `#[serde(skip)]`, so the
  counter would drop out of the snapshot and reset on load, breaking load-time hash stability.

## Sources & References

- **Parent design:** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md — §7 item 3 (line
  211), §8 Slice 5 (line 233), §6 occupancy/enter-order (line 200), critic #12 (enter-order IS
  hashed, line 262).
- **Current code:** `substrate.rs` (counter field 31, init 52), `world/mod.rs`
  (`add_entity_occupancy` 793-795, tick call 1670), `movement/movement_tick.rs` (param 826,
  assign 1316-1318, threading 1463-1464), `movement/movement_step.rs` (params 909-910, assign
  1198-1200), `movement/mod.rs` (test wrapper 281), `occupancy.rs` (rebuild sort 121),
  `game_entity.rs` (field 218, default 509), `world/world_hash.rs` (counter 49, per-entity 387).
- **Prior slices on dev:** Slice 4 `4ab1bf6`/`0d37ada`/`288ab4b`/`8cc7022`/`74e1ca0`/`69f0b2b`;
  Slice 3 `df59c36`/`bfb6cfe`/`a58e8fd`/`fc9c461`.
- **INI keys:** none.
