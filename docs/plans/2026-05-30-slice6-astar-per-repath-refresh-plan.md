# Slice 6 — A* Per-Repath Snapshot Refresh — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make each mover's A* repath pathfind against an entity-block snapshot that reflects *same-tick* committed moves (gamemd's sequential-live ordering), instead of one snapshot frozen at tick-start.

**Architecture:** The per-tick movement loop builds one entity-block snapshot per owner *before* the mover loop, then reuses it as movers mutate positions — so a unit that repaths later in the tick sees stale (start-of-tick) positions of every other mover. This plan adds a monotonic mutation counter to `OccupancyGrid` and uses it to lazily rebuild an owner's snapshot at the top of each mover iteration whenever occupancy has changed since that snapshot was built. A* itself, `AStarOptions`, and the ~10 caller layers are untouched.

**Design Doc:** `docs/research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §8 "Slice 6", drift #4 in §4.2, contract C-RECORD in §5. (This study is the approved design; the user selected the "per-repath refresh" scope on 2026-05-30.)

---

## Grounding Summary

- **What the study tells us (R1):** §4.2 drift #4 — "A* uses a precomputed entity-block snapshot, not per-neighbor live `Can_Enter_Cell`… the snapshot can go stale within a tick." §0/§1 establish gamemd processes movers in *live object order*, each reading live `CellClass` `FirstObject`/`AltObject` lists (R9–R11), and CLAUDE.md's documented tick order lists "ground movement" as a single sequential phase. The study's Slice 6 explicitly offers "(or prove the snapshot bit-equivalent on a measured scenario)" as an acceptance path.
- **Code verified live this session (R3):** `astar_search` (`src/sim/pathfinding/core.rs:821`) is a pure function over `AStarOptions` — it never reads `EntityStore`; the live classifier `classify_occupied_cell_with_layers` (`src/sim/pathfinding/cell_entry.rs:486`) already exists but only runs at movement-commit time. The snapshot is built once/owner at `src/sim/movement/movement_tick.rs:889-903`, before the mover loop at `:930`; refs are fetched at `:948-954`; the repath (`handle_path_exhaustion` → A*) fires at `:983`, *inside* the `entities.get_mut(entity_id)` scope opened at `:972-973`. `OccupancyGrid` (`src/sim/occupancy.rs:98`) is `{ cells: BTreeMap<(u16,u16), CellOccupancy> }` with **no** serde derive; mutators are `add` (`:169`), `remove` (`:206`), `move_entity` (`:216`, = remove+add), `update_sub_cell` (`:232`).
- **Key correctness argument:** A single A* search is synchronous — nothing mutates during it. Therefore a snapshot rebuilt immediately before a repath is bit-equivalent to per-neighbor live classification for that search, *except* a narrow stacked-occupant tie-break (`LayeredEntityBlockMap::insert` is last-write-wins vs a live first-in-list scan) which is **out of scope** here. This is why we do not thread `EntityStore` into the hot path.
- **Repo pattern mirrored:** `bump_crush::build_entity_block_set` (`src/sim/movement/bump_crush.rs:216`) is the existing fresh-snapshot builder used inline by the `world_commands.rs`/`world_orders.rs` order-issue paths (already fresh). We reuse it verbatim; only the *cadence* in `movement_tick.rs` changes.
- **INI (R4):** None. This slice is pure timing/architecture; no `rules(md).ini`/`art(md).ini` constants are involved.
- **Ghidra (R2):** No gamemd function is reimplemented. The only native premise consumed — movers processed sequentially-live, each reading live cell lists — is already established in the study and CLAUDE.md's tick order. Flagged MEDIUM-HIGH confidence for `/review-plan` (see Key Technical Decisions). The Rust change is observably-equivalent regardless of the premise's exact internal shape, so it is not strictly load-bearing.
- **Still unknown after grounding:** Whether the freshness fix alone fully matches gamemd in the "two units both target the *not-yet-entered* cell" case, or whether reserve-on-intent (marking a destination at move-commit) is additionally required. Deferred — measured by the integration behavior, see Open Questions.

## Key Technical Decisions

- **Per-repath refresh via an occupancy mutation-generation gate** (not a live-per-neighbor A* rewrite, not reserve-on-intent). — **Confidence:** high. **Source:** this session's reading of `core.rs`/`movement_tick.rs`/`cell_entry.rs` + the synchronicity argument + study §4.2/§5/§8.
- **`OccupancyGrid.generation: u64` as the staleness signal**, bumped in `add`/`remove`/`update_sub_cell`. `move_entity` is covered transitively. — **Confidence:** high. **Source:** `add`/`remove` are the only cell-membership mutators (`src/sim/occupancy.rs:169-238`); a unit changing cells calls `move_entity` at `src/sim/movement/movement_step.rs:1201`.
- **Counter is transient, never hashed.** `OccupancyGrid` has no serde derive and is rebuilt on load (study §4.1 "Rebuilt"); the rebuilt block-set content is a pure function of entity state, so the counter only changes *when* a deterministic rebuild happens, never *what* it produces. — **Confidence:** high. **Source:** `src/sim/occupancy.rs:98` (no derive); `OccupancyGrid::rebuild` `:111`.
- **Inject the gated refresh at the top of the mover iteration** (after `snap`, before the block refs at `:948`) — the only point in the iteration where neither `entities` nor `entity_block_sets` is otherwise mutably/immutably committed for the iteration. — **Confidence:** high. **Source:** verified borrow structure `movement_tick.rs:935-996`.
- **Native sequential-live mover premise.** — **Confidence:** medium-high (study + CLAUDE.md tick order; not re-verified live this session). **Flag for `/review-plan`.** Non-load-bearing because the fix is observably-equivalent either way.

## Open Questions

### Resolved During Planning
- *Does the fix require coupling A* to `EntityStore`?* No — fresh snapshot ≡ live per-neighbor for a synchronous search (synchronicity argument). Source: `astar_search` is pure (`core.rs:821`).
- *Does the counter affect determinism / the state hash?* No — transient, on a non-hashed rebuilt grid. Source: `occupancy.rs:98` (no serde), study §4.1.
- *Do the order-issue callers need the same fix?* No — `world_commands.rs`/`world_orders.rs` build a fresh snapshot inline per command (already fresh). Source: `bump_crush::build_entity_block_set` callsites.

### Deferred to Implementation
- *Is reserve-on-intent additionally needed for full two-movers parity?* Cannot be decided until the freshness fix is in and the two-movers scenario is measured in-game. If two units still both commit to a *not-yet-entered* shared cell identically to the stale behavior, escalate to a follow-up slice modelling destination reservation. This plan deliberately does **not** implement it (user scope choice).
- *Exact magnitude of the full-skirmish replay hash shift* — depends on how many repaths change route; observed at execution, not predictable here.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/occupancy.rs` | Add `generation: u64` counter + `generation()` accessor; bump on mutation. |
| Modify | `src/sim/movement/movement_tick.rs` | Add `refresh_owner_block_set_if_stale` helper; make `entity_block_sets` mutable; track `built_at_gen`; call the gated refresh at the top of the mover iteration. |

No files created. No file approaches the ~600-line split threshold as a result (occupancy.rs +~10 lines; movement_tick.rs +~25 lines and a small helper fn).

## Interface Changes

- `OccupancyGrid` gains a private field `generation: u64` and a public method `pub fn generation(&self) -> u64`. Additive; no existing caller breaks. The struct's public mutators (`add`/`remove`/`move_entity`/`update_sub_cell`) keep their signatures.
- New **private** fn `refresh_owner_block_set_if_stale(...)` in `movement_tick.rs` (module-internal; not exported).
- No change to `astar_search`, `AStarOptions`, `LayeredEntityBlockMap`, `EntityBlockEntry`, `cell_entry.rs`, or `bump_crush::build_entity_block_set`.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (no arithmetic added beyond a `u64` counter increment; no game-logic math).
- [x] New state included in deterministic state hash — **intentionally NO.** The `generation` counter is transient render-irrelevant scheduling state on a rebuilt-not-persisted grid; including it would be wrong. The *behavioral* change (different paths) flows through already-hashed entity/path state.
- [x] No dependencies on render/ui/sidebar/audio/net — confirmed; only `sim/` internals touched.
- [x] Tick ordering impact — within the existing "ground movement" phase only; no new phase, no reordering of `advance_tick`. Movers are still processed in `entity_order` (live object order).
- [x] BTreeMap iteration order — `entity_block_sets` and `built_at_gen` are `BTreeMap` keyed by `InternedId`; mover loop iterates `entity_order`. All deterministic.

## Risk Areas

- **State-hash shift is EXPECTED.** Units repathing later in a tick now see same-tick moves, so some computed routes change → the full-skirmish replay hash will differ from pre-change. This is the fix landing, not a regression. Acceptance is via the behavioral tests below + manual in-game confirmation, **not** hash-identity. Do not "fix" the hash change.
- **Borrow ordering.** The gated refresh mutably borrows `entity_block_sets`; the per-iteration refs (`mover_entity_blocks`/`mover_entity_block_map`) immutably borrow it for the rest of the iteration and are read inside the `get_mut(entity_id)` scope at `:995-996`. The refresh MUST complete (its `&mut` borrow end) before those refs are taken at `:948`. Task 3 places it there explicitly.
- **Working-tree drift.** `src/sim/occupancy.rs`, `src/sim/movement/movement_tick.rs`, and `src/sim/pathfinding/core.rs` had **uncommitted working-tree changes** at plan time (the user's in-progress slice work). Line numbers in this plan are against that on-disk state; re-anchor each edit by the quoted surrounding code, not the bare line number, at execution time. If a parallel session has restructured the mover loop, stop and re-scope.
- **Perf (scale target).** Worst case — every mover commits a move and a later same-owner mover repaths — is O(movers × entities) rebuilds/tick. Acceptable: the gate skips the no-op common case, and a single repath (≤65,527 nodes) dominates one O(entities) rebuild. A true all-movers-repath storm is already catastrophic independent of this change. An incremental owner-neutral delta snapshot is the future optimization toward 20k units; **not** in this slice.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 3 | Repath sees same-tick committed moves | gamemd processes movers sequentially-live; a unit repathing after an ally advances must route against the ally's NEW cell, every contended tick. Player-visible as path shape / which unit yields in group movement. | `astar_repath_sees_same_tick_committed_move` helper unit test (Task 2) + manual in-game two-units-into-one-corridor observation vs gamemd. |
| Task 3 | Refresh cadence = lazy on occupancy change | Over-refreshing wastes cycles; under-refreshing reintroduces staleness. Gate must rebuild iff occupancy changed since the owner's set was built. | `owner_block_set_*` unit tests (Task 2): rebuild on gen advance, no rebuild when gen unchanged. |
| Task 1 | Generation excluded from state hash | A hashed transient counter would break lockstep across save/load (counter resets on rebuild). | Confirm no serde derive on `OccupancyGrid`; counter defaults to 0 in `new()`; not referenced by `world_hash.rs`. |

---

## Tasks

### Task 1: Add a mutation-generation counter to `OccupancyGrid`

**Why:** Provides the cheap O(1) staleness signal the per-repath refresh gates on. Independent, foundational, fully unit-testable — done first.

**Files:**
- Modify: `src/sim/occupancy.rs` (struct `:98`, `add` `:169`, `remove` `:206`, `update_sub_cell` `:232`)

**Pattern:** New field on an existing incrementally-maintained grid; mirrors how `cells` is the single owned state. No new pattern.

**Step 1: Add the field**

In `src/sim/occupancy.rs`, change the struct (`:98`):
```rust
pub struct OccupancyGrid {
    cells: BTreeMap<(u16, u16), CellOccupancy>,
    /// Monotonic counter bumped on every cell-membership mutation. Used by the
    /// movement tick to detect when a mover's pathfinding entity-block snapshot
    /// is stale and must be rebuilt before a same-tick repath. Transient
    /// scheduling state only: never serialized, never part of the state hash —
    /// it gates *when* a deterministic rebuild happens, never *what* it produces.
    generation: u64,
}
```
And initialize it in `new()` (`:161-165`):
```rust
pub fn new() -> Self {
    Self {
        cells: BTreeMap::new(),
        generation: 0,
    }
}
```

**Step 2: Bump on every mutator + add the accessor**

At the very top of `add` (`:169`, first statement inside the body), insert:
```rust
self.generation = self.generation.wrapping_add(1);
```
Do the same as the first statement inside `remove` (`:206`) and `update_sub_cell` (`:232`). (`move_entity` calls `remove` + `add`, so it is covered transitively — do not add a separate bump there.)

Add the accessor inside `impl OccupancyGrid` (e.g., directly after `new()`):
```rust
/// Current mutation generation. Bumped on every `add`/`remove`/`update_sub_cell`
/// (and thus `move_entity`). Compare across two points to detect whether cell
/// membership changed in between. Transient — not hashed, resets to 0 on rebuild.
pub fn generation(&self) -> u64 {
    self.generation
}
```

**Step 3: Add the unit test**

In the existing `#[cfg(test)] mod tests` block in `src/sim/occupancy.rs`, add:
```rust
#[test]
fn generation_bumps_on_every_mutation() {
    let mut grid = OccupancyGrid::new();
    let g0 = grid.generation();
    grid.add(1, 1, 10, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
    let g1 = grid.generation();
    assert!(g1 > g0, "add must bump generation");
    grid.move_entity(1, 1, 2, 2, 10, MovementLayer::Ground, None, CellListInsertion::PrependNonBuilding);
    let g2 = grid.generation();
    assert!(g2 > g1, "move_entity (remove+add) must bump generation");
    grid.update_sub_cell(2, 2, 10, Some(3));
    let g3 = grid.generation();
    assert!(g3 > g2, "update_sub_cell must bump generation");
    grid.remove(2, 2, 10);
    assert!(grid.generation() > g3, "remove must bump generation");
}
```
(Confirm `MovementLayer` and `CellListInsertion` are already in scope in the test module; existing tests in this file use them — if not, add `use crate::sim::movement::locomotor::MovementLayer;` / they are imported at file top already.)

**Step 4: Verify**

Run: `cargo test -p vera20k generation_bumps_on_every_mutation -- --nocapture`
Expected: the literal line `test result: ok. 1 passed`.

**Step 5: Confirm determinism boundary**

Grep that nothing hashes the new field: `rg "occupancy" src/sim/world/world_hash.rs` returns no read of `.generation`. Confirm `OccupancyGrid` still has no `#[derive(Serialize` / `Deserialize)]` above `:98`. (It does not as of plan time — this step guards against a parallel change.)

**Step 6: Commit** (to `dev`, per repo workflow): `occupancy: add mutation-generation counter for snapshot staleness`.

---

### Task 2: Add the gated-refresh helper + its unit tests (logic first, no wiring yet)

**Why:** Isolates the staleness logic into a pure, testable function before touching the borrow-sensitive mover loop. Surfaces the core behavior (the two-movers freshness assertion) as a deterministic unit test that needs no full `Simulation`.

**Files:**
- Modify: `src/sim/movement/movement_tick.rs` (add a private fn near the existing block-set plumbing; add tests to the file's `#[cfg(test)]` module)

**Pattern:** Reuses `bump_crush::build_entity_block_set` (`src/sim/movement/bump_crush.rs:216`) verbatim — the same builder used inline by the order-issue paths. New helper is module-private.

**Step 1: Write the helper**

Add to `src/sim/movement/movement_tick.rs` (place it adjacent to `snapshot_mover`, near `:146`, so it lives with the other tick-local helpers). Use the fully-qualified types already used by the pre-loop build so no new imports are required:
```rust
/// Rebuild one owner's pathfinding entity-block snapshot iff occupancy has
/// mutated since that snapshot was last built. Returns whether a rebuild ran.
///
/// gamemd processes movers in live object order, each reading live cell lists;
/// our snapshot is built once per tick, so a later mover would otherwise pathfind
/// against pre-move positions. Gating on the occupancy generation makes the
/// snapshot match the live state at repath time (bit-equivalent to per-neighbor
/// live classification for a synchronous A* search) while skipping the no-op case.
fn refresh_owner_block_set_if_stale(
    entity_block_sets: &mut BTreeMap<
        crate::sim::intern::InternedId,
        (
            BTreeSet<(u16, u16)>,
            crate::sim::pathfinding::LayeredEntityBlockMap,
        ),
    >,
    built_at_gen: &mut BTreeMap<crate::sim::intern::InternedId, u64>,
    owner: crate::sim::intern::InternedId,
    current_gen: u64,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &crate::sim::intern::StringInterner,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> bool {
    if built_at_gen.get(&owner).copied() == Some(current_gen) {
        return false;
    }
    let owner_str = interner.resolve(owner);
    let pair = bump_crush::build_entity_block_set(entities, owner_str, alliances, interner, rules);
    entity_block_sets.insert(owner, pair);
    built_at_gen.insert(owner, current_gen);
    true
}
```
(Confirm `BTreeSet`, `BTreeMap`, `EntityStore`, `HouseAllianceMap`, `bump_crush` are already imported at the top of `movement_tick.rs` — they are, since the pre-loop build and mover collection use all of them. If `HouseAllianceMap` is referenced elsewhere only by full path, use `crate::map::houses::HouseAllianceMap` here too.)

**Step 2: Add the unit tests**

Add to the `#[cfg(test)] mod tests` block in `src/sim/movement/movement_tick.rs`. These mirror the existing test style in this file (`test_intern`, `GameEntity::test_default`, `test_interner`):
```rust
// Slice 6 acceptance: a snapshot rebuilt at repath time reflects same-tick
// moves — i.e. observably equivalent to live per-neighbor Can_Enter_Cell for a
// synchronous search. Maps to study Slice 6 tests
// `astar_repath_sees_same_tick_committed_move` / `astar_neighbor_uses_live_can_enter_cell`.
#[test]
fn owner_block_set_refreshes_when_occupancy_generation_advances() {
    use crate::map::entities::EntityCategory;
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::MovementLayer;
    use std::collections::BTreeMap;

    let interner = crate::sim::intern::test_interner();
    let alliances = HouseAllianceMap::new();
    let owner = test_intern("Americans");

    let mut entities = EntityStore::new();
    let mut blocker = GameEntity::test_default(10, "HTNK", "Americans", 5, 5);
    blocker.category = EntityCategory::Unit;
    entities.insert(blocker);

    // Initial snapshot at gen 0: friendly stationary unit -> soft-block entry at (5,5).
    let mut sets = BTreeMap::new();
    sets.insert(
        owner,
        bump_crush::build_entity_block_set(&entities, "Americans", &alliances, &interner, None),
    );
    let mut built_at: BTreeMap<crate::sim::intern::InternedId, u64> = BTreeMap::new();
    built_at.insert(owner, 0);
    assert!(sets[&owner].1.contains_key(MovementLayer::Ground, &(5, 5)));
    assert!(!sets[&owner].1.contains_key(MovementLayer::Ground, &(6, 6)));

    // Same-tick move of the blocker to (6,6); occupancy generation advances to 7.
    {
        let b = entities.get_mut(10).unwrap();
        b.position.rx = 6;
        b.position.ry = 6;
    }
    let rebuilt =
        refresh_owner_block_set_if_stale(&mut sets, &mut built_at, owner, 7, &entities, &alliances, &interner, None);
    assert!(rebuilt, "stale snapshot must rebuild when generation advances");
    assert!(!sets[&owner].1.contains_key(MovementLayer::Ground, &(5, 5)), "old cell freed");
    assert!(sets[&owner].1.contains_key(MovementLayer::Ground, &(6, 6)), "new cell blocked");
}

#[test]
fn owner_block_set_not_rebuilt_when_generation_unchanged() {
    use crate::map::entities::EntityCategory;
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use std::collections::BTreeMap;

    let interner = crate::sim::intern::test_interner();
    let alliances = HouseAllianceMap::new();
    let owner = test_intern("Americans");

    let mut entities = EntityStore::new();
    let mut blocker = GameEntity::test_default(10, "HTNK", "Americans", 5, 5);
    blocker.category = EntityCategory::Unit;
    entities.insert(blocker);

    let mut sets = BTreeMap::new();
    sets.insert(
        owner,
        bump_crush::build_entity_block_set(&entities, "Americans", &alliances, &interner, None),
    );
    let mut built_at: BTreeMap<crate::sim::intern::InternedId, u64> = BTreeMap::new();
    built_at.insert(owner, 4);

    // Generation matches the recorded build gen -> no rebuild, even if entities changed.
    entities.get_mut(10).unwrap().position.rx = 6;
    let rebuilt =
        refresh_owner_block_set_if_stale(&mut sets, &mut built_at, owner, 4, &entities, &alliances, &interner, None);
    assert!(!rebuilt, "no rebuild when generation is unchanged");
}
```
Notes for the executor: `LayeredEntityBlockMap::contains_key(layer, &cell)` is public (`src/sim/pathfinding/core.rs:201`). A friendly stationary `Unit` is recorded in the **soft** `LayeredEntityBlockMap` (code 6), not the hard `ground_blocked` `BTreeSet`; that is why the assertions probe the map's `.1`. `test_intern` and `test_interner` must be the matching pair already used in this file's tests — verify both resolve `"Americans"` consistently before relying on them.

**Step 3: Verify**

Run: `cargo test -p vera20k owner_block_set -- --nocapture`
Expected: the literal line `test result: ok. 2 passed`.

**Step 4: Commit** (to `dev`): `sim/movement: gated owner block-set refresh helper (Slice 6 logic)`.

---

### Task 3: Wire the gated refresh into the mover loop

**Why:** Integration — applies the now-tested helper at the verified safe injection point, replacing the stale start-of-tick reuse with same-tick freshness. Ordered last because it depends on Tasks 1 and 2 and is the borrow-sensitive change.

**Files:**
- Modify: `src/sim/movement/movement_tick.rs` (`:889-903` build; `:930-954` iteration head)

**Pattern:** Minimal cadence change; the snapshot data model and all downstream consumers are unchanged.

**Step 1: Make the snapshot map mutable and capture its build generation**

At the pre-loop build (`:889`), change `let entity_block_sets` to `let mut entity_block_sets`. Immediately after the `.collect();` that ends at `:903`, add:
```rust
// Generation the just-built sets reflect. Captured BEFORE process_pending_drive_arrivals
// so that if drive arrivals move any unit, the first mover that consumes a set will
// see the generation advance and rebuild (the pre-built sets are then stale).
let block_set_build_gen = occupancy.generation();
let mut block_set_built_at_gen: BTreeMap<crate::sim::intern::InternedId, u64> =
    entity_block_sets.keys().map(|&owner| (owner, block_set_build_gen)).collect();
```
`process_pending_drive_arrivals(... &entity_block_sets ...)` at `:905` still compiles — it takes `&` of the now-`mut` binding.

**Step 2: Insert the gated refresh at the top of the mover iteration**

In `for entity_id in movers {` (the second/real movement loop at `:930`), after `let Some(snap) = snapshot_mover(entities, entity_id) else { continue; };` (`:935`) and the `prone_crawls` block, and **before** the `let (mover_entity_blocks, mover_entity_block_map) = entity_block_sets.get(&snap.owner)...` fetch at `:948`, insert:
```rust
// Slice 6: rebuild this owner's pathfinding snapshot if occupancy changed since
// it was built (e.g. an earlier mover this tick committed a move). Matches
// gamemd's sequential-live ordering; no-op when nothing moved. Must run before
// the immutable refs below are taken, since it mutably borrows entity_block_sets.
refresh_owner_block_set_if_stale(
    &mut entity_block_sets,
    &mut block_set_built_at_gen,
    snap.owner,
    occupancy.generation(),
    entities,
    alliances,
    interner,
    rules,
);
```
Leave the existing `:948-954` ref fetch exactly as-is; it now reads the (possibly just-rebuilt) set.

**Step 3: Compile-check the borrows**

Run: `cargo check -p vera20k`
Expected: clean compile. If E0502 (`entity_block_sets` borrowed mutably then immutably) appears, the refresh call was placed after the ref fetch — move it above `:948`. If `occupancy` is reported mutably borrowed at the `generation()` call, confirm no `&mut occupancy` is held open at iteration top (none is, per `:935-947`).

**Step 4: Targeted behavior + regression**

Run: `cargo test -p vera20k -- movement`
Then the focused Slice 6 tests: `cargo test -p vera20k owner_block_set generation_bumps`
Expected: read the literal `test result:` lines; all pass. Investigate any *newly* failing movement test — a path-shape assertion that changes because a mover now sees a same-tick move is the **expected** improvement; update that test's expectation only after confirming the new path is the gamemd-correct one (do not loosen it blindly).

**Step 5: Full regression**

Run: `cargo test -p vera20k`
Expected: read the final `test result:` line. Pre-existing unrelated failures from the uncommitted working tree (if any) are not introduced by this change — diff against a baseline `cargo test` run on the working tree before Task 1 if unsure (see Task 0 note below).

**Step 6: Commit** (to `dev`): `sim/movement: refresh A* entity-block snapshot per repath in live order (Slice 6)`.

---

### Task 4: Verify against gamemd behavior (manual, no code)

**Why:** Confirms the observable parity goal — the reason for the slice — beyond unit tests.

**Verify:**
- **Scenario:** two friendly units ordered along the same narrow corridor toward adjacent goals, such that the trailing unit repaths while the leader advances within the same tick.
- **How:** run the engine (`/run`) and the original `gamemd.exe` side by side; observe which unit yields and the trailing unit's route when it repaths into the space the leader just vacated/entered.
- **Expected (gamemd):** the trailing unit routes against the leader's *current* cell that tick, not its tick-start cell. The Rust engine should now match.
- **Record:** if the two-units-into-one-not-yet-entered-cell case still diverges (both commit identically), that is the deferred reserve-on-intent gap (Open Questions) — note it for a follow-up slice; it is **out of scope** here.

---

### Task 0 (do this FIRST, before Task 1): Baseline the working tree

**Why:** Three target files have uncommitted changes; capture a test baseline so Task 3's regression step can distinguish pre-existing failures from new ones.

**Verify:**
- Run `cargo test -p vera20k 2>&1 | rg "test result:"` once before editing and save the summary counts. Any failure present here is pre-existing (not caused by this slice). If the build itself fails on files you did not touch, a parallel session may be mid-edit — pause and confirm with the user before proceeding (CLAUDE.md "Parallel sessions").

---

## Sources & References

- **Design doc:** `docs/research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (§0, §1, §4.1, §4.2 drift #4, §5 C-RECORD, §8 Slice 6).
- **Related study docs (navigation):** `CELLCLASS_SUBSTRATE_{FIRST_MIGRATION_SLICE,RUST_CALLER_INVENTORY,CAN_ENTER_CELL_RUNTIME_SHAPE}`, `LOGICCLASS_VS_MAPCLASS`, `SUBSTRATE_PARITY_LEDGER_20260529`.
- **Repo code (verified this session):**
  - `src/sim/pathfinding/core.rs:821` (`astar_search`, pure), `:201` (`LayeredEntityBlockMap::contains_key`), `:152`/`:166` (block entry/map types).
  - `src/sim/pathfinding/cell_entry.rs:486` (`classify_occupied_cell_with_layers`, the live classifier — commit-time only).
  - `src/sim/movement/bump_crush.rs:216` (`build_entity_block_set`, reused builder).
  - `src/sim/movement/movement_tick.rs:889-903` (pre-loop build), `:930-954` (iteration head), `:972-996` (mutable-entity scope + repath), `:146/:159` (`snapshot_mover`/`owner`).
  - `src/sim/movement/movement_step.rs:1201` (`occupancy.move_entity` at cell crossing).
  - `src/sim/occupancy.rs:98` (struct, no serde), `:169/:206/:216/:232` (mutators), `:111` (`rebuild`).
- **gamemd.exe / Ghidra:** none reimplemented; native sequential-live mover premise cited from the study + CLAUDE.md tick order (flagged for `/review-plan`).
- **INI keys:** none.
- **Build:** package `vera20k` — `cargo test -p vera20k`, `cargo check -p vera20k`. Read the literal `test result:` line before reporting pass/fail.
