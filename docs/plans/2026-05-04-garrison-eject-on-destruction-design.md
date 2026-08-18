# Garrison Eject on Destruction — Design

## Goal

When a `CanBeOccupied` building is destroyed, eject its garrison occupants alive
at random foundation cells (LIFO order, scatter on placement) instead of killing
them with the building. Mirrors gamemd `BuildingClass::SpawnSurvivors` §4a.

## Architecture Context

Garrison occupants live inside the building entity as
`PassengerRole::Transport { cargo: PassengerCargo }` — the same enum used for
vehicle transports. Today, when any cargo-holding entity dies, the death loop in
[src/sim/combat/mod.rs:415-430](../../src/sim/combat/mod.rs#L415-L430) marks
every passenger `dying = true` and clears their `PassengerRole`. There is no
distinction between garrison buildings and transports.

A **sell-path eject** already exists at
[src/sim/production/production_sell.rs:246](../../src/sim/production/production_sell.rs#L246)
(`eject_garrison_occupants`). It iterates LIFO, places occupants on foundation
**perimeter** cells, scatters via `issue_direct_move`, kills if no perimeter
cell is free (parachute system not implemented), and reverts ownership to
`garrison_original_owner`.

A **crewed-building destruction eject** pattern is established at
[src/sim/combat/mod.rs:282-290](../../src/sim/combat/mod.rs#L282-L290) +
[src/sim/world/mod.rs:1194-1205](../../src/sim/world/mod.rs#L1194-L1205): combat
collects a `DestroyedCrewedBuilding` event during the death loop, world
dispatches `production::eject_destruction_survivors` after combat. This is the
template the new feature follows.

**gamemd reference** (per `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` §4a,
`SpawnSurvivors @ 0x00442D90`):
- LIFO over occupant vector.
- Each placed at `building.center + random_foundation_offset` — random cells
  *inside* the foundation footprint, not the perimeter.
- Owner = building's current owner (`field_0x8C`, the garrisoning player).
- Successful unlimbo → Scatter mission. AI gets Mission_Hunt instead.
- Unlimbo failure → `Destroy()` (no parachute fallback on destruction; that's
  only sell).
- IC-killed (`field_0x6E0 != 0`) → bypass unlimbo entirely → ChangeOwner(attacker)
  + Destroy. (Out of scope for this design — see Deferred.)

## Impact Analysis

**Files touched:**
- [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — new `DestroyedGarrisonBuilding` struct; new field on `CombatTickResult` and `DeathEffects`; branch in death loop.
- [src/sim/world/mod.rs](../../src/sim/world/mod.rs) — new dispatch loop next to the crewed-building dispatch.
- [src/sim/production/production_sell.rs](../../src/sim/production/production_sell.rs) — new `eject_destruction_garrison` helper.

**Risk areas:**
- The current "kill all passengers" loop runs for both transports and
  garrisons. Branching incorrectly would silently change transport-death
  behavior. The branch must gate on `obj.can_be_occupied &&
  category == Structure`.
- Garrison occupants stay `PassengerRole::Inside { transport_id }` while the
  building dies and despawns. The dangling transport_id is fine as long as the
  eject helper runs before any other system queries those passengers — which
  the combat → world dispatch ordering guarantees.
- Determinism: all randomness must go through `sim.rng`. RNG draw order must be
  documented and stable.

**Out of scope (explicit):**
- Transport death — kept as current "kill all riders". gamemd has separate
  vehicle survivor logic that needs its own design.
- Iron Curtain branch — separate code path, deferred to a follow-up design.
- AI Mission_Hunt distinction — we issue Scatter for both player and AI units
  until the AI mission system lands.
- Audio event on eject (`PlayAudioEvent` in gamemd) — deferred.

## Chosen Approach

**Approach B: Deferred event mirroring `DestroyedCrewedBuilding`.**

During the combat death loop, when a destroyed entity is a `CanBeOccupied`
structure with passengers, push a `DestroyedGarrisonBuilding` event onto
`CombatTickResult` and **skip** the existing kill-passengers loop for that
building. After combat, `world/mod.rs` iterates the events and calls
`production::eject_destruction_garrison`, which handles random foundation cell
placement, scatter, and kill-on-fail fallback.

Rejected: inlining the eject in the combat death loop (pollutes combat with
spawn/scatter knowledge); generalizing the existing sell helper with a mode
flag (three divergence axes — cell strategy, fallback, ownership — make the
flag worse than honest duplication).

## Design

### Components

**1. `DestroyedGarrisonBuilding`** — new struct in `combat/mod.rs`:

```rust
/// A CanBeOccupied building destroyed in combat with live occupants — survivor
/// ejection is deferred to the caller (which has access to `Simulation` for
/// repositioning and scatter).
pub struct DestroyedGarrisonBuilding {
    pub building_id: u64,
    pub type_id: InternedId,
    pub owner: InternedId,        // ejected infantry inherit this
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
    pub foundation_w: u16,
    pub foundation_h: u16,
    pub passenger_ids: Vec<u64>,
}
```

**2. `CombatTickResult.destroyed_garrison_buildings`** — new collection plumbed
through `DeathEffects` exactly like `destroyed_crewed_buildings`.

**3. `production::eject_destruction_garrison`** — new helper in
`production_sell.rs`:

```rust
pub fn eject_destruction_garrison(
    sim: &mut Simulation,
    rules: &RuleSet,
    event: &DestroyedGarrisonBuilding,
) -> usize;  // returns count successfully ejected
```

### Data Flow

```
combat::process_deaths (death loop)
  for &dead_id in dead_entities:
    let entity = entities.get(dead_id);
    let obj = rules.object(entity.type_ref);
    if obj.can_be_occupied
       && entity.category == Structure
       && cargo.has_passengers:
        destroyed_garrison_buildings.push(DestroyedGarrisonBuilding { ... });
        // SKIP the existing passenger-kill loop for this building
        continue_after_kill_block();
    else:
        // existing kill-all-passengers loop runs (transports unchanged)

world::advance_tick (after combat)
  for ev in &combat_result.destroyed_garrison_buildings:
      production::eject_destruction_garrison(self, rules, ev);

production::eject_destruction_garrison
  shuffle foundation cell offsets via sim.rng
  for &pax_id in event.passenger_ids.iter().rev():    // LIFO
      pick first cell from shuffled list not in used_cells
      if no cell available:
          mark passenger dying; continue
      reposition passenger at chosen cell
      clear PassengerRole::Inside → PassengerRole::None
      set owner = event.owner
      register in occupancy grid
      issue scatter move to random NEIGHBORS direction (sim.rng)
      ejected += 1
  return ejected
```

### Interfaces / Contracts

- **Death-loop branch position:** the new branch sits *before* the current
  `passenger_ids` kill loop in `combat/mod.rs:415-430`. On match, the death loop
  pushes the event and skips the kill block via early `continue` or guarded
  block. Transport behavior (kill all riders) is preserved by the `else` arm.
- **Helper signature:** `fn eject_destruction_garrison(&mut Simulation, &RuleSet, &DestroyedGarrisonBuilding) -> usize`. Pure side-effects on `sim.entities` and `sim.occupancy`. No new fields on `Simulation`.
- **Cell selection:** enumerate foundation offsets `(dx, dy)` for `dx in 0..foundation_w, dy in 0..foundation_h`. Shuffle once with `sim.rng`. Per occupant, walk the shuffled list and take the first cell not in `used_cells`.
- **Owner:** snapshot of `building.owner` taken when the event is created. The `garrison_original_owner` is intentionally ignored — no ownership-revert semantics apply when the building is gone.
- **Scatter:** reuse the existing `NEIGHBORS` array and `issue_direct_move` from the sell path. Random direction via `sim.rng.next_u32() as usize % 8`.

### Error Handling

- **Empty cargo:** event is not pushed (death loop guards on `!cargo.passengers.is_empty()`).
- **Zero-area foundation:** defensive — helper marks all occupants dying. Should not occur for a building that successfully held occupants.
- **All foundation cells blocked by other entities:** matches gamemd
  "Destroy on unlimbo failure" — mark remaining occupants `dying = true`,
  `health.current = 0`, `passenger_role = None`. The death-animation system
  cleans them up.
- **Passenger entity missing (already despawned by another system):** skip
  silently. `entities.get_mut` returns `None` → continue.

### Testing Strategy

Place in `src/sim/passenger.rs` tests module (existing garrison test
infrastructure) or a new `combat/garrison_eject_tests.rs`:

1. **Happy path:** spawn a 2x2 `CanBeOccupied` building, garrison 3 GIs, deal
   lethal damage, advance one tick.
   - Assert building despawned.
   - Assert 3 GIs alive at distinct cells within foundation footprint.
   - Assert each GI's owner = the original garrisoning player.
   - Assert each GI has `PassengerRole::None` and a scatter movement target.
2. **Blocked foundation:** pre-occupy all 4 foundation cells with other
   entities, kill building, advance tick.
   - Assert occupants marked `dying = true`, `health.current = 0`.
3. **Transport unchanged:** spawn an APC with riders, destroy it, advance tick.
   - Assert riders still die. (Negative test that the new branch only fires for
     `CanBeOccupied`.)
4. **Determinism:** two `World::advance_tick` runs from identical snapshots
   produce identical occupant placements and scatter directions. Existing state
   hash assertions cover this implicitly.

### Determinism

- All randomness via `sim.rng`. RNG call order, documented in the helper's doc
  comment:
  1. Foundation cells shuffled once per building (Fisher-Yates over `w*h`).
  2. Per occupant (LIFO): one `next_u32` for scatter direction.
- State hash already covers entity `position`, `owner`, `passenger_role`, and
  `movement_target` — all touched by the eject. No new hash inputs needed.
- Tick ordering: combat phase produces `CombatTickResult`; the immediate
  post-combat block in `world/mod.rs` already runs in the existing combat slot
  of `World::advance_tick`. No reordering of phases.

## Architectural Decisions

- **Follows the `DestroyedCrewedBuilding` pattern verbatim.** Same struct
  shape, same plumbing site, same dispatch hook, same destination module
  (`production_sell.rs`). Zero new architectural patterns.
- **Honest duplication over mode flags.** The destruction-eject helper
  duplicates ~30 lines of LIFO/scatter primitives with `eject_garrison_occupants`,
  but the two paths diverge on three axes (cell strategy: interior vs perimeter;
  fallback: kill vs parachute; ownership: inherit vs revert). A mode flag would
  hide the divergence; separate functions make it explicit.
- **No new patterns introduced.** Helper signature, event lifetime, and
  RNG-draw discipline all match existing sim conventions.

## Alternatives Considered

- **Approach A — inline in combat death loop:** rejected. Combat module would
  need to import spawn/scatter helpers and reach into `Simulation`-level state,
  breaking the "death loop produces deferred effects, world layer applies
  them" pattern.
- **Approach C — mode-flagged single helper:** rejected. The sell vs destruction
  paths diverge on three independent axes (cell selection, fallback, ownership
  revert). A mode flag forces every reader to hold both paths in their head,
  and the upcoming IC branch on the destruction path would make it worse.

## Deferred (Follow-up Designs)

- Iron Curtain garrison-eject branch (`ChangeOwner(attacker) + Destroy`).
- Transport-death survivor logic (separate gamemd `SpawnSurvivors` path for
  vehicles).
- AI Mission_Hunt vs player Scatter distinction (depends on AI mission system).
- Audio event on garrison eject (`PlayAudioEvent` in gamemd).
