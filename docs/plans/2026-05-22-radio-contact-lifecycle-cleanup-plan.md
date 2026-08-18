# Radio Contact Lifecycle Cleanup Implementation Plan

> Execute task-by-task. Each task should build independently. Do not commit unless the user explicitly asks for commits.

**Goal:** Clear RadioClass-style `GameEntity.radio_contacts` at verified Techno-like removal and limbo-like boundaries so stale live contacts cannot survive despawn, death, sell, crush, passenger hide, or aircraft drop retry paths.

**Architecture:** Add a deterministic `EntityStore` contact-scrub primitive, expose a small `Simulation` wrapper for sim-level callers, and route verified lifecycle paths through those APIs. This avoids a broad RadioClass rewrite and keeps the fix inside `sim/`, where `radio_contacts` already live and are included in the state hash.

**Design Doc:** [docs/plans/2026-05-22-radio-contact-lifecycle-cleanup-design.md](2026-05-22-radio-contact-lifecycle-cleanup-design.md)

---

## Grounding Summary

**Binary/research:** `RADIO_SYSTEM_MODEL_SYNTHESIS.md` and the underlying Ghidra reports document RadioClass sparse contact slots, reciprocal BREAK cleanup, and Techno limbo broadcasting `BREAK(3)` before conceal. The cleanup entry-point report identifies Rust direct-removal and limbo-like paths that currently bypass generic `radio_contacts` cleanup.

**Rust mismatch:** `GameEntity.radio_contacts` is stored per entity and included in the deterministic state hash in `src/sim/world/world_hash.rs`, but `Simulation::despawn_entity` and several direct `entities.remove` paths do not clear peer references.

**Implementation scope:** Close the high-priority generic contact cleanup gap. Do not implement a full RadioClass dispatcher, airfield FIFO/CachedDock behavior, service-depot radio handoff, or Hospital/Armory legacy service behavior in this plan.

## Key Technical Decisions

- **Add the primitive to `EntityStore`, then wrap it on `Simulation`.** Combat and movement crush cleanup only have `&mut EntityStore`; forcing those systems to take `&mut Simulation` would be a much larger signature change. `EntityStore` is still inside `sim/`, deterministic, and already owns the data being scrubbed.
- **Use `retain` on existing vectors.** This preserves first-observed contact order for all remaining contacts and keeps state hashing deterministic.
- **Make cleanup idempotent.** Missing ids, one-sided contacts, and empty vectors are no-ops. This matches existing contact helper behavior and makes it safe to call at multiple lifecycle boundaries.
- **Clean before removal or hide.** The leaving entity should still be present when possible so its own vector can be cleared before physical removal or `PassengerRole::Inside` transition.
- **Leave reservation cleanup separate.** Refinery dock contacts, building dock reservations, and aircraft dock reservations remain their own systems.

## Open Questions

### Resolved During Planning

- **Techno-like only or every entity id?** Use the evidence-backed lifecycle scope, but make the helper generic over stable ids.
- **Where should the helper live?** Store-level primitive plus Simulation wrapper. This fits both direct `EntityStore` call sites and higher-level sim callers.
- **Should wall direct removal be changed?** No code change planned for wall removal. There is no verified wall RadioClass contact path; changing wall removal would broaden the parity surface without evidence.

### Deferred

- Airfield FIFO/CachedDock radio parity.
- Service depot radio handoff and chrono rejection parity.
- Full RadioClass command dispatcher.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/entity_store.rs` | Add deterministic `clear_radio_contacts_for` helper and focused unit tests |
| Modify | `src/sim/world/mod.rs` | Add `Simulation::clear_radio_contacts_for`, call from `despawn_entity` |
| Modify | `src/app_sim_tick.rs` | Use `sim.despawn_entity` or a sim lifecycle helper for completed death-animation removal |
| Modify | `src/sim/production/production_sell.rs` | Clear contacts before sold building entity removal |
| Modify | `src/sim/combat/mod.rs` | Clear contacts before immediate structure/voxel death removal |
| Modify | `src/sim/movement/movement_tick.rs` | Clear contacts before deferred crush removal |
| Modify | `src/sim/passenger.rs` | Clear passenger contacts before setting `PassengerRole::Inside` |
| Modify | `src/sim/aircraft/drop_payload.rs` | Clear passenger contacts before attach-failed retry restores `Inside` |
| Modify | tests near touched modules | Add regression coverage for one-sided, reciprocal, removal, and hide transitions |

## Interface Changes

- `EntityStore::clear_radio_contacts_for(stable_id: u64)`:
  - removes `stable_id` from every entity's `radio_contacts`;
  - clears `stable_id`'s own `radio_contacts` if the entity still exists;
  - is safe when `stable_id` is absent.
- `Simulation::clear_radio_contacts_for(stable_id: u64)`:
  - thin wrapper around the store helper for lifecycle code that owns `&mut Simulation`.

No snapshot/schema change is required. `radio_contacts` already exists and remains hashed.

## Sim Checklist

- [x] No render/ui/sidebar/audio/net dependency from `sim/`.
- [x] Deterministic iteration considered: `EntityStore` is `BTreeMap`; helper uses sorted store iteration and order-preserving `retain`.
- [x] State hash impact understood: cleared contacts change the same hashed state that currently records stale contacts.
- [x] Tick ordering preserved: cleanup happens at removal/hide boundary, not at earlier target marking or damage calculation.
- [x] No new INI, asset, or floating-point gameplay logic.

## Risk Areas

- **Borrowing:** `clear_radio_contacts_for` must not hold a mutable borrow to the leaving entity while iterating all entities. Implement it as one pass over `values_mut()`; the leaving entity will be included naturally.
- **Combat/movement signatures:** These systems do not have `&mut Simulation`; use the `EntityStore` helper directly.
- **Owned-count semantics:** Do not replace every removal with `despawn_entity` blindly. Combat deaths and sell paths already manage ownership/pathgrid side effects differently.
- **App-side removal:** `app_sim_tick.rs` currently mutates `sim.entities` directly after death animation. Route through `despawn_entity` or a dedicated sim lifecycle helper so app code does not bypass sim cleanup.
- **Over-cleaning:** Do not clear unrelated contacts on the transport when a passenger enters. Only scrub references to the passenger id and clear the passenger's own vector.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | One-sided contacts are scrubbed | Rust currently creates one-sided war-factory mover contacts; cleanup must not assume reciprocal pairs | EntityStore unit test |
| 1 | Reciprocal contacts are scrubbed | gamemd BREAK clears both sides of a live contact | EntityStore unit test |
| 2 | `despawn_entity` clears contacts | Central sim removal path must not leave stale ids in peer vectors | Simulation unit test |
| 3 | Direct immediate removals clear contacts | Sell, combat immediate death, and crush bypass `despawn_entity` today | Focused module tests or existing fixture extensions |
| 4 | Passenger hide clears contacts before `Inside` | Techno limbo-like conceal broadcasts BREAK before hidden state | Passenger boarding test |
| 5 | Aircraft attach-failed retry clears contacts | Retry re-hides the passenger inside aircraft cargo | Drop payload test |
| 6 | Death-animation final removal does not bypass sim cleanup | App layer should not mutate hashed sim state directly | Focused app/sim test or code inspection plus `cargo check` |

---

## Tasks

### Task 1: Add `EntityStore::clear_radio_contacts_for`

**Why:** This is the primitive every hard-to-reach direct removal path can use without changing broad system signatures.

**Files:**
- Modify: `src/sim/entity_store.rs`

**Implementation:**

Add a public method on `EntityStore`:

```rust
/// Clear all RadioClass-style live contacts involving `stable_id`.
///
/// Idempotent. Safe if `stable_id` is absent.
pub fn clear_radio_contacts_for(&mut self, stable_id: u64) {
    for entity in self.entities.values_mut() {
        entity.clear_live_contact_with(stable_id);
        if entity.stable_id == stable_id {
            entity.radio_contacts.clear();
        }
    }
}
```

**Tests:**

Add tests in `entity_store.rs`:

- one-sided contact `1 -> 2` is removed when clearing `2`;
- reciprocal contact `1 <-> 2` is removed from both when clearing `1`;
- missing id does not change unrelated contacts;
- remaining contact order is preserved, e.g. `[2, 3, 4]` clearing `3` becomes `[2, 4]`.

**Verify:**

Run:

```powershell
cargo test --lib sim::entity_store
```

Expected: PASS.

### Task 2: Add `Simulation::clear_radio_contacts_for` and wire `despawn_entity`

**Why:** Higher-level sim code should not reach into `entities` for lifecycle cleanup.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Implementation:**

Add near `despawn_entity`:

```rust
pub(crate) fn clear_radio_contacts_for(&mut self, stable_id: u64) {
    self.entities.clear_radio_contacts_for(stable_id);
}
```

Then call it inside `despawn_entity` before `self.entities.remove(stable_id)`.

Do not change owned-count logic or occupancy removal ordering except for inserting contact cleanup before removal.

**Tests:**

Add a minimal world/simulation test near existing world tests if present, or in `world_hash.rs` if that is where current contact hash tests live:

- create two entities with contacts;
- insert into `Simulation`;
- call `despawn_entity`;
- assert peer no longer has the despawned id.

**Verify:**

Run:

```powershell
cargo test --lib live_radio_contacts
cargo check --workspace
```

Expected: PASS.

### Task 3: Replace verified direct removal bypasses

**Why:** The high-priority disparity is that direct removals bypass RadioClass BREAK-style cleanup.

**Files:**
- Modify: `src/sim/production/production_sell.rs`
- Modify: `src/sim/combat/mod.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Implementation:**

In `production_sell.rs`, before:

```rust
sim.entities.remove(stable_id);
```

call:

```rust
sim.clear_radio_contacts_for(stable_id);
```

In `combat/mod.rs`, before immediate removal:

```rust
entities.remove(dead_id);
```

call:

```rust
entities.clear_radio_contacts_for(dead_id);
```

In `movement_tick.rs`, before crush removal:

```rust
entities.remove(victim_id);
```

call:

```rust
entities.clear_radio_contacts_for(victim_id);
```

Do not route these paths through `despawn_entity`; their ownership, sound, occupancy, and death-result side effects are already specialized.

**Tests:**

At minimum:

- sell-building test with a peer contact to the sold building, assert peer contact cleared;
- combat immediate death test with contact to destroyed structure/voxel entity;
- crush test with contact to crushed victim.

If focused fixtures are expensive, add lower-level tests around the functions that already exercise these paths rather than introducing full skirmish setup.

**Verify:**

Run:

```powershell
cargo test --lib production_sell
cargo test --lib combat
cargo test --lib movement
cargo check --workspace
```

Expected: PASS.

### Task 4: Clear contacts on passenger hide/inside transitions

**Why:** gamemd's Techno limbo path broadcasts BREAK before conceal. Rust boarding hides the passenger by setting `PassengerRole::Inside` but currently leaves `radio_contacts` intact.

**Files:**
- Modify: `src/sim/passenger.rs`

**Implementation:**

In the production boarding path, immediately before:

```rust
pax.passenger_role = PassengerRole::Inside { transport_id };
```

call:

```rust
sim.clear_radio_contacts_for(pax_id);
```

Make sure the call happens before borrowing the passenger mutably for the role change.

Do not clear all transport contacts. The helper will remove only references to the passenger id.

**Tests:**

Add or extend a passenger boarding test:

- passenger has contact with a building/producer;
- peer has contact back to passenger;
- after successful boarding, passenger role is `Inside`;
- passenger contacts are empty;
- peer no longer contains passenger id.

**Verify:**

Run:

```powershell
cargo test --lib sim::passenger
cargo check --workspace
```

Expected: PASS.

### Task 5: Clear contacts on aircraft drop attach-failed retry

**Why:** The retry path re-hides a passenger inside aircraft cargo. That is a limbo-like transition and was called out by the cleanup audit.

**Files:**
- Modify: `src/sim/aircraft/drop_payload.rs`

**Implementation:**

Before the attach-failed retry restores:

```rust
passenger.passenger_role = PassengerRole::Inside {
    transport_id: aircraft_id,
};
```

call:

```rust
sim.clear_radio_contacts_for(passenger_id);
```

Do this before taking the mutable borrow of the passenger.

**Tests:**

If `drop_payload.rs` already has a retry-path test, extend it. Otherwise add a focused test that forces `begin_parachute_descent` failure or exercises the attach-failed branch through the existing public helper.

Assert:

- passenger is restored to `Inside`;
- passenger contacts are empty;
- peer contacts no longer contain passenger id.

**Verify:**

Run:

```powershell
cargo test --lib drop_payload
cargo test --lib aircraft
cargo check --workspace
```

Expected: PASS.

### Task 6: Route app-side death-animation final removal through sim lifecycle cleanup

**Why:** `app_sim_tick.rs` currently removes completed death-animation entities directly from `sim.entities`, bypassing sim lifecycle cleanup for hashed state.

**Files:**
- Modify: `src/app_sim_tick.rs`

**Implementation:**

Replace the direct final removal:

```rust
sim.entities.remove(*dead_id);
```

with a sim lifecycle call. Preferred if compatible with existing behavior:

```rust
sim.despawn_entity(*dead_id);
```

If double occupancy removal is a concern, either remove the preceding manual occupancy removal or add a narrow helper that only does contact cleanup plus entity removal. For death-animation entities, `despawn_entity` should not decrement owned counts because they are already marked `dying`.

**Tests:**

If app-level tests exist for death animation cleanup, extend them. If not, rely on a focused code inspection plus full compile/test pass, because this path may be hard to isolate without app scaffolding.

**Verify:**

Run:

```powershell
cargo check --workspace
cargo test --workspace
```

Expected: PASS.

### Task 7: Integration regression test for stale radio contacts

**Why:** Unit tests cover helper behavior, but the bug is a lifecycle integration bug.

**Files:**
- Prefer adding tests in modules that already own fixtures:
  - `src/sim/world/world_hash.rs`
  - `src/sim/production/production_tests.rs` or `production_placement_tests.rs`
  - `src/sim/passenger.rs`

**Test cases:**

1. Create one-sided war-factory-style contact from mover to producer. Remove producer through the chosen lifecycle path. Assert mover no longer has producer id.
2. Create reciprocal contacts. Remove one side through `despawn_entity`. Assert no surviving entity references removed id.
3. Put contacted passenger into transport. Assert contacts are cleared while passenger remains in `EntityStore`.
4. Confirm `state_hash` after cleanup matches an equivalent simulation that never had the stale removed contact.

**Verify:**

Run:

```powershell
cargo test --lib live_radio_contacts
cargo test --lib radio_contacts
cargo test --workspace
```

Expected: PASS.

### Task 8: Final verification

**Why:** This touches shared lifecycle state and deterministic hashing.

Run:

```powershell
cargo fmt --all
cargo clippy --all-targets
cargo test --workspace
```

Expected: PASS.

Then grep for remaining risky direct removals:

```powershell
rg -n "entities\\.remove\\(" src/sim src/app_sim_tick.rs
```

Review each remaining hit:

- acceptable if test-only/helper-only;
- acceptable if not a Techno-like/contact-capable lifecycle path, such as wall removal;
- otherwise route through contact cleanup or document why not.

## Sources & References

- **Design doc:** [docs/plans/2026-05-22-radio-contact-lifecycle-cleanup-design.md](2026-05-22-radio-contact-lifecycle-cleanup-design.md)
- **Disparity scan:** [docs/gap-scans/2026-05-22-disparity-scan-radio-system.md](../gap-scans/2026-05-22-disparity-scan-radio-system.md)
- **Synthesis:** `docs/research/RADIO_SYSTEM_MODEL_SYNTHESIS.md`
- **Ghidra reports:** `GENERIC_DESPAWN_LIMBO_CLEANUP_ENTRY_POINTS_GHIDRA_REPORT.md`, `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`
- **Current Rust surfaces:** `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/passenger.rs`, `src/sim/aircraft/drop_payload.rs`, `src/app_sim_tick.rs`
