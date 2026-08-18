# Radio Contact Lifecycle Cleanup Design

## Goal

Clear RadioClass-style live contacts deterministically when Techno-like entities are removed, hidden, or otherwise enter limbo-like states.

## Architecture Context

Rust currently stores live radio contacts on each `GameEntity` as `radio_contacts: Vec<u64>`. Contacts are idempotent and order-preserving through `mark_live_contact_with`, checked by systems such as movement/building entry, and included in the simulation state hash. This makes stale contacts both a gameplay bug and a deterministic-state bug.

The main lifecycle choke point is `Simulation::despawn_entity`, but several production paths remove entities directly through `entities.remove`: sell-building removal, immediate combat death, crush death, wall damage removal, app-side death-animation completion, and some test/helper paths. Passenger boarding and aircraft drop retry do not remove the entity, but they hide the passenger in a transport-style state where gamemd's limbo behavior clears live radio contacts.

Existing reservation-specific cleanup is separate: refinery dock contacts, building dock reservations, and airfield docks have their own dead-id cleanup. Those do not clear generic `GameEntity.radio_contacts` and should not be treated as a substitute for RadioClass BREAK/limbo cleanup.

Layering constraint: the cleanup primitive belongs in `sim/`, because `radio_contacts` are simulation state and are hashed. Higher app/render code should call a sim-owned lifecycle API rather than mutating `sim.entities` directly when finalizing entity death.

## Impact Analysis

Primary touched modules:

- `src/sim/game_entity.rs`: keep the existing contact helpers; no new dependency needed.
- `src/sim/world/mod.rs`: add a sim-owned contact cleanup helper and thread it into `despawn_entity`.
- `src/sim/production/production_sell.rs`: clear contacts before sold building removal.
- `src/sim/combat/mod.rs`: clear contacts before immediate structure/voxel vehicle removal.
- `src/sim/movement/movement_tick.rs`: clear contacts before crush removal, either through a passed callback/API or a localized helper path.
- `src/sim/passenger.rs`: clear contacts before passenger entities enter `PassengerRole::Inside`.
- `src/sim/aircraft/drop_payload.rs`: clear contacts when attach-failed retry re-hides a passenger inside the aircraft.
- `src/app_sim_tick.rs`: replace final `sim.entities.remove` for completed death animations with a sim lifecycle API.

Risk areas:

- Tick ordering: cleanup must happen at the same removal/limbo boundary, not earlier during damage marking.
- Determinism: iteration must stay deterministic. `EntityStore` is `BTreeMap`, and `retain` preserves existing contact order.
- Borrowing/API shape: direct callers that currently receive only `&mut EntityStore` may need either a small helper on the store, a deferred cleanup list, or a narrow Simulation wrapper.
- Scope creep: wall/overlay entities should not become a new radio feature. If a direct wall removal goes through a generic cleanup API, that is defensive only; the parity target remains Techno-like contacts.

## Chosen Approach

Use one central sim-owned lifecycle cleanup primitive and route verified removal/limbo-like call sites through it.

Recommended API shape:

- `Simulation::clear_radio_contacts_for(stable_id: u64)`:
  - removes `stable_id` from every other entity's `radio_contacts`;
  - clears the removed/hidden entity's own `radio_contacts` if it still exists;
  - is deterministic because `EntityStore` iteration is ordered and each contact vector uses stable `retain`.
- `Simulation::remove_entity_after_lifecycle_cleanup(stable_id: u64)` or equivalent:
  - wraps `clear_radio_contacts_for`;
  - performs existing occupancy/count behavior where appropriate;
  - becomes the preferred removal path for Techno-like sim removals.
- For non-removal limbo-like transitions, call `clear_radio_contacts_for(pax_id)` immediately before changing `PassengerRole` to `Inside`.

This keeps RadioClass cleanup in `sim`, follows the existing per-entity contact model, and avoids encoding radio behavior into reservation systems or app/render code.

## Tiny-Detail Ledger

- Radio contacts are sparse contact slots and BREAK/HELLO manage reciprocal relationships; cleanup must remove both the leaving id from peers and peers from the leaving entity. Source: `RADIO_SYSTEM_MODEL_SYNTHESIS.md`, backed by `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`.
- Techno limbo broadcasts `BREAK(3)` before conceal; Rust passenger hide/inside transitions should clear contacts at the role-change boundary. Source: `RADIO_SYSTEM_MODEL_SYNTHESIS.md`; Ghidra addresses `0x0065AA80`, `0x0065ACE0`, `0x0065A970`, `0x0065A820`.
- Rust direct removals currently bypass generic contact cleanup. Source: `GENERIC_DESPAWN_LIMBO_CLEANUP_ENTRY_POINTS_GHIDRA_REPORT.md` and local code scan of direct `entities.remove` paths.
- `radio_contacts` participate in the deterministic state hash, so cleanup order and contact vector ordering are lockstep-relevant. Source: `src/sim/world/world_hash.rs`.
- Contact vectors are idempotent and first-observed-order preserving. Cleanup must use `retain` rather than sorting or rebuilding in a nondeterministic order. Source: `src/sim/game_entity.rs`.
- Reservation cleanup is not generic RadioClass cleanup. Refinery, building dock, and aircraft dock dead-id cleanup must remain separate from live contact cleanup. Source: `RADIO_SYSTEM_MODEL_SYNTHESIS.md` and disparity scan `docs/gap-scans/2026-05-22-disparity-scan-radio-system.md`.
- Refinery dock entry already has specific contact/reservation handling and should not be rewritten as part of this cleanup. Source: disparity scan false-positive ledger.
- War-factory spawn contact currently creates a one-sided mover-to-producer contact in Rust. Cleanup must handle one-sided and reciprocal contacts without assuming both sides were marked. Source: `src/sim/production/production_spawn.rs`.
- App-side death-animation final removal currently mutates `sim.entities` directly. Since contacts are sim state, final removal must cross a sim lifecycle API boundary. Source: local code scan of `src/app_sim_tick.rs`.
- Stock Hospital/Armory walk-in service should not be added as part of this work; verified docs mark that path as legacy/conditional, not a standard YR cleanup requirement. Source: `RADIO_SYSTEM_MODEL_SYNTHESIS.md`.

## Design

### Components

Add a small lifecycle cleanup surface on `Simulation`, probably near `despawn_entity` in `src/sim/world/mod.rs`:

- contact cleanup helper;
- removal wrapper for direct removals that should behave like limbo/despawn;
- tests covering reciprocal, one-sided, and hidden-passenger cleanup.

No new subsystem is needed. This is lifecycle hygiene for an existing field.

### Interfaces / Contracts

`clear_radio_contacts_for(stable_id)` contract:

- safe if `stable_id` is missing;
- safe if no contacts exist;
- removes all peer references to `stable_id`;
- clears `stable_id`'s own contacts if present;
- does not touch dock reservations, cargo lists, movement targets, attack targets, or ownership counts.

Removal wrapper contract:

- should call the cleanup before physical removal;
- should preserve existing caller-specific side effects such as occupancy removal, owned-count decrement, refund/sell logic, garrison ejection, death events, and sound events.

Passenger hide contract:

- before setting `PassengerRole::Inside`, clear that passenger's radio contacts;
- do not clear the transport's unrelated contacts unless they mention the passenger id.

### Data Flow

1. A lifecycle path decides an entity is leaving the active world or entering hidden passenger state.
2. The path calls the sim-owned cleanup helper with the entity stable id.
3. The helper scrubs peer contact vectors and the entity's own vector.
4. The existing path continues with role change or entity removal.
5. `state_hash` reflects the cleared contacts in the same tick as the removal/hide boundary.

### Error Handling

No fallible API is needed. Missing entities and missing contacts are no-ops, matching the current idempotent contact helpers.

### Testing Strategy

Focused sim tests:

- reciprocal contacts are cleared from both entities before despawn/removal;
- one-sided war-factory-style contact is cleared when either side is removed;
- passenger boarding clears passenger contacts and peer references;
- aircraft drop attach-failed retry clears passenger contacts before restoring `Inside`;
- completed death-animation removal uses the sim lifecycle helper and clears contacts;
- state hash changes when stale contacts are cleared, and does not retain removed ids.

Regression tests should construct minimal `Simulation` entities and avoid requiring full asset loads unless the existing module test pattern already uses rules fixtures.

## Architectural Decisions

- Keep cleanup in `sim/` because contacts are deterministic simulation state.
- Do not add a RadioClass subsystem yet. The current field is sufficient for the verified cleanup gap.
- Do not merge dock-reservation cleanup with generic radio cleanup. They represent different contracts.
- Keep the helper generic over stable ids, but apply it at Techno-like verified lifecycle boundaries. This avoids hardcoding category assumptions while keeping the parity scope evidence-backed.

## Alternatives Considered

### Local retain calls at every removal site

Rejected. It is easy to miss future direct-removal paths, creates duplicated lifecycle behavior, and makes app-side mutation more likely to drift from sim rules.

### Full RadioClass command dispatcher first

Rejected for this fix. The broader radio system has more message semantics, but the verified high-priority bug is lifecycle cleanup. A full dispatcher would delay a player-visible stale-contact fix and touch more behavior than needed.

### Global scrub after every tick

Rejected. It would hide lifecycle bugs, move cleanup timing away from the gamemd BREAK/limbo boundary, and add unnecessary per-tick work.

## Handoff

This design is ready for a focused implementation plan or direct implementation after approval. The implementation should stay scoped to lifecycle cleanup and tests; airfield FIFO/CachedDock and service depot radio handoff remain separate radio-system gaps.
