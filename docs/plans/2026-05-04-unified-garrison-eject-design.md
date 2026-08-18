# Unified Garrison Eject Path Design

## Goal

Route both Sell and Combat-Destruction of a garrisoned building through one
eject helper, matching gamemd's single `SellBuilding @ 0x00457DE0` code path.
Resolves the v2 disparity-scan finding **N1** (Sell-on-captured-civilian
demolishes) and the baseline **G1** (combat-death kills occupants inline).

## Architecture Context

Three call sites are involved today:

- **Sell command path** —
  [src/sim/world/world_commands.rs:479-485](src/sim/world/world_commands.rs#L479-L485)
  validates ownership (`entity_owned_by_id`), calls
  [src/sim/production/production_sell.rs::sell_building (376-421)](src/sim/production/production_sell.rs#L376-L421).
- **`sell_building`** — calls
  [`eject_garrison_occupants` (246-373)](src/sim/production/production_sell.rs#L246-L373)
  to atomically eject occupants (LIFO with foundation-edge search, scatter
  direction from `sim.rng`, kill if no exit cell, conditionally revert
  ownership via `garrison_original_owner`). Then unconditionally
  `entities.remove` and pays refund.
- **Combat death path** — `handle_entity_deaths` at
  [src/sim/combat/mod.rs:415-430](src/sim/combat/mod.rs#L415-L430) inline-kills
  every cargo entry (`pax.health.current = 0; pax.dying = true`) before
  building despawn. The crewed-survivor sibling
  [`eject_destruction_survivors`](src/sim/production/production_sell.rs#L184-L227)
  gates on `obj.crewed` only and never checks `obj.can_be_occupied`, so
  garrison occupants fall through and die.

**Key signals already on `GameEntity`:**

- `garrison_original_owner: Option<InternedId>` — `Some` ⇔ captured civilian
  (was Neutral/Special pre-garrison). Auto-revert at
  [passenger.rs:614](src/sim/passenger.rs#L614) `.take()`s this when cargo
  empties on unload, so `is_some()` ⇔ "captured AND has occupants".
- `obj.can_be_occupied` (parsed from `CanBeOccupied=yes`) — building is
  garrison-capable. Distinguishes garrison buildings from vehicle transports.

**Existing helper does the right thing for both paths.**
`eject_garrison_occupants` already implements gamemd's atomic LIFO eject
with foundation-edge search, scatter, and kill-if-no-exit. The conditional
owner revert is harmless on the death path (entity is removed next anyway).
The main work is wiring it into the death path and adding the
"skip-removal-skip-refund" branch on the sell path.

**EVA cue context.** `SimSoundEvent::StructureAbandoned` is currently
emitted only from `tick_unloading` at
[passenger.rs:606-622](src/sim/passenger.rs#L606-L622) when the last occupant
leaves via player-driven Unload. The audio consumer drops EVA events on the
floor at
[app_building_anim.rs:446-450](src/app_building_anim.rs#L446-L450) (v2 scan
finding **N3**) — so EVA cues are currently silent regardless. The sim event
is still emitted; only the playback is gated.

## Impact Analysis

**Touched files:**

- `src/sim/production/production_sell.rs` —
  - Refactor `eject_garrison_occupants` signature from `&mut Simulation` to
    split borrows: `(entities, occupancy, rng, rules, interner, building_id)`.
  - Make it `pub(crate)` so combat can call it.
  - Add captured-civilian branch in `sell_building`: detect
    `garrison_original_owner.is_some()`, capture pre-revert owner, call
    helper, push `StructureAbandoned`, early-return BEFORE
    `entities.remove`/refund.
- `src/sim/combat/mod.rs` —
  - Add `rng: &mut DeterministicRng` to `handle_entity_deaths` and
    `tick_combat_with_fog`.
  - In `handle_entity_deaths`, call `eject_garrison_occupants` for
    `obj.can_be_occupied` buildings BEFORE the inline-kill loop. Helper
    clears cargo, so the kill-loop becomes self-cancelling (no explicit
    branch needed).
- `src/sim/world/mod.rs::advance_tick` — pass `&mut sim.rng` into
  `tick_combat_with_fog`. One-line change.
- Test modules for the 6 new tests (see Testing Strategy).

**Blast radius:**

- `tick_combat_with_fog` signature change. Callers grep small (1-2 sites,
  all in sim/).
- No new fields on `GameEntity`. No INI parsing changes. No new
  `SimSoundEvent` variants. No snapshot serialization changes.
- Determinism preserved (no new RNG sources, just plumbing the existing
  `sim.rng` through one more call layer).

**Risk areas:**

- Refactoring `eject_garrison_occupants` to split borrows might reveal a
  borrow-checker conflict if `sell_building`'s outer scope holds a stale
  immutable ref. Contained to one function.
- The "inline-kill loop becomes no-op for garrisonables" is an implicit
  contract — relies on the helper having cleared `cargo.passengers` before
  the loop iterates. Mitigated by a code comment + a regression test
  (`combat_death_of_vehicle_transport_still_kills_passengers`) ensuring
  vehicle-transport behavior is unchanged.

## Chosen Approach

**Approach A (atomic three-branch in `sell_building`).** Selected over the
alternatives (route-to-`OrderIntent::Unloading`; reject-at-validator + cursor
change) because:

- **Parity:** gamemd's `SellBuilding` ejects atomically in one frame.
  Routing to `OrderIntent::Unloading` would eject one pax per tick — visible
  drift with multi-occupant civilians.
- **Locality:** all sell variants live in one function. The validator
  doesn't need to second-guess command routing.
- **Cursor parity:** Sell ($) cursor remains visible on captured civilians
  (matches gamemd's UX where the sell button works but the building isn't
  destroyed).

## Design

### Components

**1. Helper — `eject_garrison_occupants`** (refactored, location unchanged
in `production_sell.rs`)

```rust
pub(crate) fn eject_garrison_occupants(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rng: &mut DeterministicRng,
    rules: &RuleSet,
    interner: &StringInterner,
    building_id: u64,
) -> usize
```

Body unchanged from current implementation. Reads building, walks foundation
perimeter for exit cells, ejects in LIFO order, scatters via `rng`, kills
if no cell available, clears cargo, conditionally reverts
`garrison_original_owner`. Returns the count actually ejected (excludes
those killed for lack of space).

**2. Sell path — three-branch `sell_building`**

Pseudocode (real implementation lives at production_sell.rs:376):

```
fn sell_building(sim, rules, stable_id) -> bool {
    1. Validate Structure, fetch (owner_name, type_id, position, health).
    2. obj = rules.object(&type_id)?;
    3. is_captured = entity.garrison_original_owner.is_some();

    4. IF is_captured:
       // Captured civilian — eject + revert + keep building.
       abandoning_owner = entity.owner;  // pre-revert
       eject_garrison_occupants(
           &mut sim.entities, &mut sim.occupancy, &mut sim.rng,
           rules, &sim.interner, stable_id,
       );  // helper reverts owner internally
       sim.sound_events.push(StructureAbandoned { owner: abandoning_owner });
       log::info!("Building {} evacuated by {} — {} ejected, structure reverted to civilian",
                  type_id, owner_name, ejected);
       return true;  // NO entities.remove, NO refund

    5. ELSE: existing flow unchanged
       refund = sell_refund_for_building(obj, health);
       crew_ejected = eject_sell_survivors(...);          // Crewed=yes
       garrison_ejected = eject_garrison_occupants(...);  // alive before demolition
       sim.entities.remove(stable_id);
       // SpySat reshroud, refresh SWs, pay refund
       return true;
}
```

**Edge case — `is_captured` with empty cargo:** unreachable in practice
(auto-revert at passenger.rs:614 clears `garrison_original_owner` when
cargo empties; once cleared, `entity_owned_by_id` rejects Sell because
owner is back to civilian). Helper is no-op on empty cargo so the branch
is also defensively safe.

**3. Death path — call helper before inline kill-loop**

In `handle_entity_deaths` at combat/mod.rs:415-430:

```rust
// Before the existing inline-kill loop:
let is_garrisonable = rules
    .object(interner.resolve(type_id))
    .map_or(false, |o| o.can_be_occupied);
if is_garrisonable {
    // Eject garrison occupants (atomic LIFO with foundation-edge search).
    // Survivors that don't fit are killed by the helper itself.
    // The helper clears cargo, so the kill-loop below becomes a no-op
    // for garrisoned buildings — vehicle transports are unaffected.
    eject_garrison_occupants(entities, occupancy, rng, rules, interner, dead_id);
}

// Existing kill-loop, unchanged. For garrison buildings, cargo is now
// empty so this iterates 0 times. For vehicle transports (Passengers>0,
// !can_be_occupied), kills cargo as today.
let passenger_ids = entities.get(dead_id)
    .and_then(|e| e.passenger_role.cargo())
    .map(|c| c.passengers.clone())
    .unwrap_or_default();
for &pid in &passenger_ids {
    if let Some(pax) = entities.get_mut(pid) {
        pax.health.current = 0;
        pax.dying = true;
        pax.passenger_role = PassengerRole::None;
        pax.attack_target = None;
        pax.movement_target = None;
        pax.selected = false;
    }
}
```

The inline-kill loop is left in place rather than gated, because it's a
no-op for garrisonable buildings (helper cleared the cargo) and required
for vehicle transports (matches gamemd's transport explosion = kill all).

### Interfaces / Contracts

- `eject_garrison_occupants` (pub(crate)): atomic LIFO eject. Returns
  count successfully placed at exit cells (excludes those killed for
  lack of space). Idempotent on empty cargo.
- `tick_combat_with_fog`: new `rng: &mut DeterministicRng` parameter,
  threaded through `handle_entity_deaths`.
- `SimSoundEvent::StructureAbandoned`: now also pushed from
  `sell_building`'s captured-civilian branch (in addition to existing
  `tick_unloading` site).

### Data Flow

**Sell on captured civilian:**

```
SellBuilding cmd → world_commands.rs:479 (entity_owned_by_id passes)
  → production::sell_building → is_captured=true branch
    → eject_garrison_occupants  (LIFO eject + scatter + revert owner)
    → push StructureAbandoned {owner: pre-revert}
    → return  (building stays, no refund)
```

**Sell on player-built garrison:**

```
SellBuilding cmd → ... → sell_building → is_captured=false branch
  → eject_sell_survivors  (Crewed=yes random spawn, unchanged)
  → eject_garrison_occupants  (alive eject; revert is no-op since
                               garrison_original_owner is None)
  → entities.remove + SpySat reshroud + refund  (unchanged)
```

**Combat death of garrison building:**

```
damage → tick_combat_with_fog → handle_entity_deaths
  → for each dead_id:
    → if can_be_occupied: eject_garrison_occupants (LIFO + scatter)
    → inline kill-loop (no-op for garrisonable, kills cargo for vehicles)
    → AoE explosion / explosion anim / occupancy.remove / entities.remove
```

### Error Handling

- Missing entity / type / object → helper returns 0 (early-return at top).
- No exit cells available → individual occupants killed in-place
  (`health.current = 0; dying = true`). Parachute fallback deferred per
  project state; this matches today's sell-path behavior.
- Borrow-checker contention from split-borrow refactor → contained to one
  function; rebuild fix is mechanical.

### Determinism

All sources used by the helper are deterministic:

- LIFO order: `Vec::iter().rev()` on a deterministically-ordered
  `cargo.passengers` (boarding order is itself deterministic per
  `BTreeMap` iteration in tick_boarding).
- Exit cell search: `sell_survivor_positions` is a deterministic sorted
  iteration around the foundation perimeter.
- Scatter direction: `sim.rng.next_u32()`, seeded from sim state.
- Occupied-cells snapshot: `entities.values()` over `BTreeMap` —
  deterministic by `stable_id`.

No new RNG draws are introduced; only an existing draw moves from being
sell-only to also fire on combat death. This **changes the RNG draw count
per tick** when a garrison building dies in combat. State hash will differ
from pre-change replays of any save where a garrison building dies in
combat. Breaking change for replays of existing saves; acceptable.

### Testing Strategy

**Existing tests must keep passing:**

- `passenger.rs` — `test_first_occupant_emits_garrisoned_event`,
  `test_second_occupant_emits_no_garrison_event`,
  `test_last_occupant_emits_abandoned_event_with_pre_revert_owner`,
  `test_non_garrison_transport_emits_no_garrison_events`.
- `production_placement_tests.rs` —
  `sell_building_refunds_half_current_value_and_ejects_allied_infantry`,
  `sell_building_uses_owner_appropriate_survivor_type_and_caps_count`.

The first existing sell test in particular pins the
"player-built-garrison-demolish-with-refund" branch — must stay green.

**New tests:**

1. **`sell_captured_civilian_ejects_reverts_and_keeps_building`** — Setup:
   CanBeOccupied building owned by Americans with
   `garrison_original_owner = Some(Neutral)`, two occupants, credits=1000.
   Action: `sell_building(...)`. Asserts:
   - Building still in EntityStore.
   - `building.owner == Neutral`.
   - `building.garrison_original_owner == None` (helper consumed it).
   - `cargo.passengers.is_empty()`.
   - Both occupants placed at exit cells, `role == None`,
     `dying == false`, `health.current > 0`.
   - Credits == 1000 (no refund paid).

2. **`sell_captured_civilian_emits_structure_abandoned_with_pre_revert_owner`** —
   Same setup. Assert `SimSoundEvent::StructureAbandoned { owner: Americans }`
   in `sim.sound_events` (mirror of
   `test_last_occupant_emits_abandoned_event_with_pre_revert_owner`).

3. **`sell_player_built_garrisoned_building_demolishes_and_ejects_alive`** —
   Setup: player-built CanBeOccupied building (owner=Americans,
   `garrison_original_owner = None`), three occupants, credits=0,
   building cost=1000, full HP. Action: `sell_building(...)`. Asserts:
   - Building removed from EntityStore.
   - All three occupants placed at exit cells alive
     (`role == None`, `dying == false`, `health.current > 0`).
   - Credits paid (refund > 0, exact value per existing formula).

4. **`combat_death_of_garrisoned_building_ejects_occupants`** — Setup:
   CanBeOccupied building, two occupants, surrounded by free cells.
   Apply lethal damage via `tick_combat_with_fog` (or directly to
   `health.current`). Asserts:
   - Occupants placed at exit cells, `role == None`, `dying == false`,
     `health.current > 0`.
   - Building removed.

5. **`combat_death_of_garrisoned_building_with_no_exit_cells_kills_occupants`** —
   Setup: CanBeOccupied building, two occupants, surrounded by blocking
   entities. Apply lethal damage. Asserts:
   - Occupants `dying == true`, `health.current == 0` (parachute fallback
     deferred — kill is the documented graceful degradation).

6. **`combat_death_of_vehicle_transport_still_kills_passengers`** — Setup:
   non-CanBeOccupied transport (`Passengers=5`), three passengers. Apply
   lethal damage. Asserts:
   - Passengers `dying == true`, `health.current == 0` (existing
     vehicle-transport behavior preserved — guards against accidental
     routing through the eject path).

## Architectural Decisions

**Patterns followed:**

- Split-borrow refactor of `eject_garrison_occupants` matches the existing
  pattern for sim/ helpers that operate on EntityStore + adjacencies (e.g.,
  movement helpers, occupancy helpers).
- Cross-module call within sim/ (combat → production helper) is consistent
  with current usage (combat already produces `DestroyedCrewedBuilding`
  consumed by production-side `eject_destruction_survivors` from the world
  layer).
- `SimSoundEvent` push in sim/ with rendering-side consumption — unchanged.

**Patterns deviated from:** none.

**Tech debt introduced:**

- The "inline kill-loop is self-cancelling for garrisonables" relies on the
  helper having cleared cargo. A future refactor that reorders these or
  splits them could break the contract silently. Mitigation: code comment
  + regression test #6.

## Alternatives Considered

**Approach B — Route Sell-on-civilian to `OrderIntent::Unloading`.**
Validator detects captured civilian, sets `OrderIntent::Unloading` instead
of calling `sell_building`. Existing `tick_unloading` handles eject + auto-
revert + StructureAbandoned at one-pax-per-tick.

- Pros: less new code; reuses existing tick_unloading machinery.
- Cons: multi-tick eject vs gamemd's atomic SellBuilding. Visible drift
  with multi-occupant civilians (staggered ejection). Player notices.
- Rejected on parity grounds.

**Approach C — Reject at validator, change cursor.** Validator rejects
SellBuilding on captured civilians; cursor system shows a non-Sell cursor
(Eject? blank?) so the player can't try.

- Pros: most explicit UX.
- Cons: deviates from gamemd's "Sell button works on captured civilians,
  just doesn't demolish" behavior. Requires cursor changes
  (`app_cursor.rs`) and validator changes that aren't otherwise needed.
  Higher blast radius.
- Rejected on parity + scope grounds.

**Helper location alternative — Move to `passenger.rs` or new
`sim/garrison.rs`.** Considered separating eject as a passenger-lifecycle
event in `passenger.rs`, or creating a new module.

- Pros: cleaner module separation (eject is a passenger concern).
- Cons: churn for no behavioral benefit. Helper is currently in
  `production_sell.rs` and the sell-path call site lives there too.
  Combat-path can call cross-module within sim/ as it already does for
  `DestroyedCrewedBuilding` / `eject_destruction_survivors`.
- Rejected on YAGNI grounds — leave helper where it is.

**EVA-on-destruction alternative — Emit `StructureAbandoned` on combat
death.** Considered emitting the EVA cue when a garrisoned building is
destroyed.

- gamemd's `SellBuilding` plays per-infantry audio events (the unlimboed
  occupant's voice) but does NOT push a separate EVA cue on destruction.
  EVA "Structure Abandoned" fires from `CheckAutoSellOrCivilian` on the
  auto-sell branch (last occupant leaves), not from destruction.
- Decision: do NOT emit `StructureAbandoned` on combat death. Currently
  moot anyway (N3: EVA is hardcoded silent at the consumer). One-line
  revert if user disagrees later.
