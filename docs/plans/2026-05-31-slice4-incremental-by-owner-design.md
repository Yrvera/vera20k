# Slice 4 — Incremental `by_owner` Index + Owner-Change Chokepoint Design

## Goal
Make `EntityStore.by_owner` maintained incrementally (insert / remove / `change_owner`)
and delete the per-tick full rebuild, routing all live owner mutations through one
`change_owner` chokepoint — bit-identical replay hash, no `SNAPSHOT_VERSION` bump.

## Architecture Context

- **`EntityStore`** (`src/sim/entity_store.rs`) wraps `BTreeMap<u64, GameEntity>` plus a
  secondary `by_owner: BTreeMap<InternedId, Vec<u64>>`. Today `insert`/`remove` do **not**
  touch `by_owner`; it is rebuilt wholesale by `rebuild_owner_index()` (clears, then
  re-scans all entities, producing sorted Vecs because the primary map iterates ascending
  stable_id).
- **Per-tick rebuild**: `Simulation::advance_tick` calls
  `self.substrate.entities.rebuild_owner_index()` once at the top of every tick
  (`mod.rs:1644`) — an O(N) scan every tick.
- **Deserialize finalizer**: `EntityStore`'s `Deserialize` impl (`entity_store.rs:161-170`)
  builds the primary map directly then calls `rebuild_owner_index()`. Snapshot load reaches
  the store through this path.
- **`by_owner` consumers**: **none live.** Every `ids_for_owner()` call in the tree is in
  `entity_store.rs` unit tests. So the per-tick rebuild currently produces an index nothing
  reads — pure dead cost — and `by_owner` ordering cannot affect the state hash today.
- **Owned-counts** (`HouseState.owned_building_count` / `owned_unit_count`,
  `house_state.rs:38,40`) are a **separate** per-house tally, **hashed**
  (`world_hash.rs:136-137`), maintained by `increment_owned_count` / `decrement_owned_count`
  (`mod.rs:904,918`). They are independent of `by_owner`.
- **Spawn / despawn lifecycle** (post-Slice-3): `place_spawned` (`world_spawn.rs`) →
  `entities.insert` (+ for active spawns `reveal` + `increment_owned_count` + occupancy);
  `uninit` (`mod.rs:937`) → `decrement_owned_count` + occupancy-remove + conceal +
  `entities.remove`.
- **Live owner-MUTATION sites** (existing entity changes owner, distinct from spawn):
  1. **Engineer capture** — `world_orders.rs:233-242`: `b.owner = engineer_owner` via
     `get_mut`, **then** `decrement_owned_count(old)` + `increment_owned_count(new)`.
  2. **Garrison reconciliation** — `passenger.rs:600` and `passenger.rs:611`:
     `building.owner = new_owner` / `civilian_owner` directly, **no** owned-count calls.
  (All other `.owner =` writes in the tree are `#[cfg(test)]` helpers or spawn-time field
  init, not post-spawn mutations — to be re-confirmed exhaustively at plan time.)

## Impact Analysis

**Files touched (sim/ only):**
- `src/sim/entity_store.rs` — `insert`/`remove` maintain `by_owner` incrementally; add
  `change_owner(id, new_owner)`; update the module doc comment (it currently states
  insert/remove do NOT sync the index); update/extend unit tests that assert the old
  stale-until-rebuild contract.
- `src/sim/world/mod.rs` — delete the per-tick `rebuild_owner_index()` call (1644); add a
  `Simulation::change_owner` wrapper (sets owner via the store chokepoint); keep
  `rebuild_owner_index` reachable for the deserialize finalizer.
- `src/sim/world/world_orders.rs` — engineer capture routes `b.owner =` through
  `change_owner`; **keeps** its existing `decrement/increment_owned_count` calls.
- `src/sim/passenger.rs` — garrison reconcile routes both `building.owner =` writes through
  `change_owner`; **adds no** count calls (preserves current no-count behavior verbatim).

**Depends on what we change:** nothing live reads `by_owner`, so the index change has zero
behavioral blast radius. The risk surface is entirely "does any *other* live path mutate
`entity.owner` directly and thereby desync the now-incremental index?"

**Determinism / hash:** `by_owner` is not hashed and has no live consumer → cannot move the
hash. owned-counts are hashed but their maintenance is **unchanged** (same calls, same
sites). Deleting the per-tick rebuild changes no observable state. Expected: replay hash
bit-identical, no `SNAPSHOT_VERSION` bump.

**Tick ordering:** the deleted rebuild ran at tick top before command application; removing
it cannot change ordering of anything downstream because nothing consumed its output.

**Migration:** none. No serialized layout change (by_owner was never serialized;
deserialize still finalizes via rebuild).

## Chosen Approach

**EntityStore owns the incremental index.** `insert` adds the id to its owner's Vec in
sorted position; `remove` drops it; `change_owner(id, new)` moves the id between buckets and
writes `entity.owner`. The per-tick rebuild is deleted. Deserialize keeps `rebuild_owner_index`
as a finalizer (it bulk-loads the primary map directly, bypassing `insert`). All live
owner-mutation sites call `Simulation::change_owner`, which delegates to the store; owned-count
calls stay inline at each site (unchanged), because the two live sites legitimately differ in
count behavior and unifying them would change the hash.

Rejected alternatives:
- **Hook add/remove explicitly onto `unlimbo`/`uninit`** instead of `insert`/`remove` — more
  call-surface, and `insert`/`remove` are the true single funnels (every spawn/despawn goes
  through them), so hooking them is tighter and harder to bypass.
- **Dirty-flag the rebuild** (rebuild only when an owner-write happened) — keeps the
  stale-window semantics and the scattered direct owner-writes; pays down no debt.

## Tiny-Detail Ledger

This is a determinism/refactor slice (no new gamemd-matching behavior), so the ledger is the
set of invariants that keep it a true no-op:

- **`by_owner` Vec order = ascending stable_id**, identical to `rebuild_owner_index` output
  (which relies on BTreeMap ascending iteration). Incremental `insert` MUST insert at the
  sorted position (binary-search insert), not push-to-end, so `deserialize-rebuild ≡
  incremental`. [src: entity_store.rs:145-152 rebuild loop]
- **`remove` preserves remaining order** (retain / position-remove, no reorder). [src:
  entity_store.rs rebuild contract]
- **`change_owner` is order-preserving** in both source and destination buckets (remove from
  old by position, sorted-insert into new). [invariant for rebuild-equivalence]
- **owned-counts unchanged**: engineer capture still does exactly one decrement(old
  Structure) + one increment(new Structure); garrison reconcile still does **zero** count
  calls. No count call moves into `change_owner`. [src: world_orders.rs:240-242;
  passenger.rs:599-613 — hashed via world_hash.rs:136-137]
- **owned-counts are NOT derived from `by_owner`** — they remain the independent HouseState
  tally. `change_owner` must not recompute counts from index sizes. [src: house_state.rs:38,40]
- **Deserialize still finalizes via `rebuild_owner_index`** — the primary map is built
  directly in `Deserialize`, so the incremental hooks never fire on load; the explicit
  rebuild stays. [src: entity_store.rs:161-170]
- **No live `entity.owner` write may bypass `change_owner`** once the index is incremental —
  else the index silently desyncs. The plan MUST exhaustively audit live (non-test,
  non-spawn-init) `.owner =` writes and confirm only engineer-capture + garrison-reconcile
  exist, OR route any others found. Candidates to classify live-vs-test/spawn at plan time:
  `genetic_converter.rs`, `superweapon/lightning_storm.rs`, `aircraft/paradrop_mission.rs`,
  and the mind-control path (does MC change `owner` or only set `mind_controlled`?).
  [src: grep of `\.owner =` across src/sim]
- **`change_owner` is a no-op-safe on same-owner** (new == old): must not duplicate the id in
  the bucket. Garrison reconcile already early-returns on `new_owner == current_owner`
  (passenger.rs:596), but `change_owner` should be internally idempotent regardless.
- **Absent entity id**: `change_owner` on a missing id is a no-op (mirrors `get_mut` None
  guards at the call sites).

## Design

### Components
- **`EntityStore` (owner of the index).** New private helpers `index_add(owner, id)` /
  `index_remove(owner, id)` (sorted insert / position remove). `insert` calls `index_add`
  after storing; `remove` calls `index_remove` using the removed entity's owner. New public
  `change_owner(&mut self, id, new_owner)`: look up entity; if present and
  `new_owner != entity.owner`, `index_remove(old)`, set `entity.owner`, `index_add(new)`.
- **`Simulation::change_owner` wrapper** (`mod.rs`): thin delegate to
  `self.substrate.entities.change_owner(id, new_owner)`, so callers above the store don't
  reach into `substrate.entities` directly (keeps the chokepoint greppable). `pub(crate)`.

### Interfaces / Contracts
- `EntityStore::insert(entity) -> u64` — now also indexes. (Signature unchanged.)
- `EntityStore::remove(id) -> Option<GameEntity>` — now also de-indexes. (Signature unchanged.)
- `EntityStore::change_owner(&mut self, id: u64, new_owner: InternedId)` — **new.** Index +
  owner field only; no counts.
- `Simulation::change_owner(&mut self, id: u64, new_owner: InternedId)` — **new** delegate.
- `EntityStore::rebuild_owner_index(&mut self)` — retained, used only by the deserialize
  finalizer now (no longer per-tick).

### Data Flow
- **Spawn**: `place_spawned` → `entities.insert` → `index_add`. (No separate index step.)
- **Despawn**: `uninit` → `entities.remove` → `index_remove`.
- **Engineer capture**: `tick_capture_orders` → `sim.change_owner(building, new)` →
  store moves index + sets owner; then existing `decrement/increment_owned_count` as today.
- **Garrison reconcile**: `reconcile_civilian_garrison_owner_for_building` →
  `sim.change_owner(building, new)` (both the occupy-transfer and the empty→civilian revert);
  no count calls (unchanged).
- **Load**: snapshot deserialize → `EntityStore::deserialize` builds map → `rebuild_owner_index`.

### Error Handling
All new ops are total: missing id → no-op; same-owner → no-op; empty bucket cleanup on
remove (drop the `Vec`/leave empty — match `rebuild` which simply omits empty owners; an
emptied bucket should be removed from the map so `ids_for_owner` of a wiped-out house returns
`&[]` identically to a fresh rebuild).

### Testing Strategy
- **EntityStore unit tests** (rewrite the stale-contract ones):
  - `insert` indexes immediately (replaces `insert_does_not_auto_sync_owner_index`).
  - `remove` de-indexes immediately.
  - `change_owner` moves the id, preserves order, is idempotent on same-owner, no-op on
    missing id (replaces `test_owner_transfer_captured_by_rebuild`'s stale-window assertion).
  - **incremental ≡ rebuild**: build a store via incremental ops, clone, `rebuild_owner_index`
    the clone, assert `by_owner` equal for every owner (the headline acceptance).
- **Determinism**: full `cargo test -p vera20k --lib` green; replay-hash / world_hash /
  snapshot tests unchanged; per-tick membership + presence asserts (Slices 1-2) don't fire.
- **Capture / garrison regressions**: existing engineer-capture and garrison-reconcile tests
  stay green (counts unchanged; owner transfer still observed). Add one asserting
  `ids_for_owner(new)` returns the captured building's id **immediately after**
  `change_owner` with **no** rebuild (the intentional staleness fix — now there's a live
  guarantee even though no production code consumes it yet).
- **Save/load**: deserialize-rebuild path test stays green (e.g.
  `saveload_occupancy_list_order_matches_incremental` exercises rebuild_caches_after_load).

## Architectural Decisions
- **Follows** the Slice 3 pattern: a single funnel (`insert`/`remove`/`change_owner`) owns a
  cross-cutting invariant, replacing scattered hand-maintenance — same shape as `place_spawned`
  centralizing the spawn 4-step.
- **Deviates** from the current "index rebuilt, never incremental" doc contract — the module
  doc and tests asserting that are updated, not worked around.
- **Tech debt addressed**: §7 item 4 (the `by_owner` deferred-rebuild cache) and the scattered
  direct `entity.owner =` writes for live transfers.
- **Tech debt deferred (named)**: garrison ownership transfer not adjusting owned-counts is
  preserved verbatim (hash-identical requirement). Whether that matches gamemd is a separate
  hash-changing parity investigation (spawned as a follow-up task) — **not** in this slice.

## Alternatives Considered
- **Hook `unlimbo`/`uninit` explicitly** rather than `insert`/`remove`: rejected — wider
  surface, bypassable; `insert`/`remove` are the true funnels.
- **Dirty-flagged rebuild**: rejected — retains stale-window semantics and the scattered
  owner-writes; no debt paydown.
- **`change_owner` adjusts owned-counts (flag or always)**: rejected for this slice — garrison
  transfers don't adjust counts today, so unifying counts into `change_owner` changes the
  hash. Counts stay inline; the garrison-count question is a separate slice.
