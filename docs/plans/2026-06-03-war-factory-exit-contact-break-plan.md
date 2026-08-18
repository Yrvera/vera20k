# War-Factory Exit Radio-Contact Transient Break (Slice 7d) Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Package is `vera20k`
> (`cargo test -p vera20k`). Commit after each task directly to `dev`.

**Goal:** Break the war-factory↔newborn-vehicle radio contact when the vehicle clears the
factory footprint (gamemd's per-cell-process behavior), instead of holding it until despawn.

**Architecture:** A new post-ground-movement maintenance sweep in `sim/` mirrors the existing
`tick_gate_runtimes` pattern: it reads `dock_entered_with` (the `+0x418` analog already set for
refinery dockers) + occupancy and breaks the WF exit contact once the unit is off the factory
footprint. The sole reader (`build_live_building_entry_skip_map`) is untouched.

**Design Doc:** [docs/plans/2026-06-03-war-factory-exit-contact-break-design.md](docs/plans/2026-06-03-war-factory-exit-contact-break-design.md)

---

## Grounding Summary

- **Docs (this session, all `ghidra/verified`):** `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md`
  (contact created at exit; Can_Enter_Cell row-skip reader), `STOCK_REFINERY_RADIO_0X08_GLOBAL_SENDERS`
  + `RADIO_0X18_CONTACT_FLAG_LIFECYCLE` (the per-cell-process `0x08` break + cascade).
- **Ghidra confirmed live this session:** `ExitObject_Main @ 0x00443C60` (WeaponsFactory branch:
  HELLO `0x02` + `0x18` + Queue_Mission `0x10`); `FUN_0044D880` case 0 (unit `Assign_Mission(5=Guard)`);
  `UnitClass::PerCellProcess @ 0x00739EC0` send site `0x0073A93D` (break gate = `+0x418` set AND
  mission ∉ {7,0x10} AND **no building under current cell**); `TechnoClass::Receive_Radio @ 0x006F4AB0`
  case 8 (`0x19` then `0x03` BREAK → removes factory from unit `Contacts[]`).
- **Repo pattern mirrored:** `tick_gate_runtimes` (`src/sim/gate_runtime.rs:175`) — a per-tick
  maintenance sweep called right after ground movement (`src/sim/world/mod.rs:1838`), collect-ids-
  then-mutate shape.
- **INI:** no new keys. Behavior keyed off already-parsed `Factory=UnitType`, `Naval`, `ExitCoord`
  (via `exact_land_vehicle_exit_factory`).
- **Still unknown:** none blocking. gamemd's mission gate is modeled as the producer-type
  discriminator (output-equivalent; see Key Technical Decisions).

## Key Technical Decisions

- **Model `+0x418` faithfully (Approach A):** set `dock_entered_with = Some(producer)` at WF exit;
  break clears it + the contact. — **Confidence:** high. **Source:** Ghidra `0x00443C60`/`0x006F4AB0`;
  user-approved design.
- **Discriminator = producer is a WeaponsFactory land-vehicle exit factory** (not gamemd's literal
  `mission ∉ {7,0x10}` gate). Output-equivalent for all stock content (WF unit = Guard + WF contact →
  break; refinery miner = Enter/Unload + refinery contact → not a WF contact → no break), and avoids
  depending on the not-yet-authoritative `MissionCom` enum (Slice 8). — **Confidence:** high.
  **Source:** Ghidra `0x00739EC0` (gate), `0x0044D880` (Guard), design doc.
- **Footprint-clear = no `Structure`-category occupant at the unit's current cell** (matches gamemd
  `Look_up_building_in_cell == 0`, *any* building). Resolved via `OccupancyGrid::get` →
  `blockers(Ground)` → entity category. — **Confidence:** high. **Source:** Ghidra `0x00739EC0`;
  `src/sim/occupancy.rs`.
- **Placement: post-ground-movement sweep** (after `tick_gate_runtimes`), not an in-movement-loop
  per-cell hook. Output-identical for a unit that starts on the footprint (first clear-cell tick =
  cell-entry tick) and avoids multiple commit sites / borrow churn. — **Confidence:** high.

## Open Questions

### Resolved During Planning
- *Does this shift `SLICE6_BASELINE_HASH`?* **No (expected).** The slice6 scenario
  ([slice6_retask_tests.rs:75](src/sim/world/slice6_retask_tests.rs:75)) is MTNK×2 + E1 — no war
  factory, no production — so the setter and sweep never fire and `dock_entered_with`/`radio_contacts`
  stay empty. No struct field is added (`dock_entered_with` is already hashed). Task 4 verifies the
  constant is unchanged; a shift would signal a bug, not a routine re-baseline.
- *Where to home the sweep?* New file `src/sim/production/war_factory_exit.rs` —
  `production_spawn.rs` is already 832 lines (over the ~600 guideline).

### Deferred to Implementation
- None.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/production/production_spawn.rs` | Set `dock_entered_with` at WF exit; make `exact_land_vehicle_exit_factory` `pub(super)` |
| Create | `src/sim/production/war_factory_exit.rs` | `tick_war_factory_exit_contacts` sweep |
| Modify | `src/sim/production/mod.rs` | Declare + export the new module |
| Modify | `src/sim/world/mod.rs` | Call the sweep after `tick_gate_runtimes` |
| Modify | `src/sim/production/production_tests.rs` | Extend setter test + add 3 sweep tests |

## Interface Changes

- New public fn `crate::sim::production::tick_war_factory_exit_contacts(&mut EntityStore,
  &OccupancyGrid, &RuleSet, &StringInterner)`. Consumed only by `world/mod.rs::advance_tick`.
- `exact_land_vehicle_exit_factory` visibility `fn` → `pub(super) fn` (production-module-internal).
  No callers outside the module change.
- `mark_war_factory_spawn_contact` body gains one line; signature unchanged → its caller
  (`production_queue.rs:565`) is unaffected.

## Sim Checklist
- [x] No f32/f64 — sweep uses only `u16`/`u64`/`Option` and category enums.
- [x] No new hashed field — `dock_entered_with` and `radio_contacts` are already hashed; only their
  *values/timing* change.
- [x] No dependency on render/ui/sidebar/audio/net — sweep imports only `map::entities`, `rules`,
  `sim::{entity_store,intern,movement::locomotor,occupancy}`.
- [x] Tick ordering: adds one step immediately after ground movement (mirrors `tick_gate_runtimes`),
  before air/special movement — does not reorder existing phases.
- [x] BTreeMap iteration order: sweep iterates `entities.values()` (stable_id order) → deterministic;
  no RNG.

## Risk Areas
- **Reader untouched:** `build_live_building_entry_skip_map` is not edited → the passability tests
  (`empty_tank_bunker_is_passable_occupied_blocks`,
  `refinery_live_skip_map_opens_bib_east_edge_not_interior`,
  `refinery_contact_number_rows_opens_first_clear_column_only`, gate tests) must stay green unchanged.
- **Refinery lifecycle:** the discriminator (producer must be a WeaponsFactory land factory) ensures
  the refinery miner's `dock_entered_with = Some(refinery)` is never broken by this sweep. Covered by
  a dedicated test.
- **Determinism:** `determinism_replay` must stay green; `SLICE6_BASELINE_HASH` must stay unchanged
  (Task 4).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | `dock_entered_with` set at WF exit (`+0x18`→`+0x418`) | gamemd sets the byte at exit; the break gate reads it | Ghidra `0x00443C60`; new setter assertion |
| 2 | Break trigger = first cell with no Structure under it (footprint clear) | gamemd `0x0073A93D` gate; controls exactly when row-skip ends | Ghidra `0x00739EC0`; `breaks_when_unit_clears_footprint` / `held_while_on_footprint` tests |
| 2 | Discriminator = WeaponsFactory producer (protects refinery) | wrong scope would break refinery docks | Ghidra `0x00739EC0` mission gate equivalence; `ignores_non_weapons_factory_producer` test |
| 2 | Break removes producer from `radio_contacts` (+ clears `dock_entered_with`) | models `0x08→0x19→0x03`; ends the row-skip | Ghidra `0x006F4AB0` case 8; sweep tests |
| 3 | Sweep runs after ground movement, same tick | matches PerCellProcess timing (gone by T+1) | placement after `tick_gate_runtimes`; determinism_replay |
| 4 | `SLICE6_BASELINE_HASH` unchanged; `determinism_replay` green | lockstep correctness | Task 4 run |

---

## Tasks

### Task 1: Set `dock_entered_with` at the war-factory exit (the `+0x418` model)

**Why:** gamemd's `ExitObject_Main` sends `0x18` to the produced unit (`+0x418 = 1`) alongside the
HELLO contact. Modeling it makes the break gate (Task 2) faithful and supplies the per-unit marker.

**Files:**
- Modify: `src/sim/production/production_spawn.rs:213` (setter body) and `:217` (helper visibility)
- Modify: `src/sim/production/production_tests.rs:708-762` (extend the existing assertion)

**Pattern:** mirrors the refinery, which sets `dock_entered_with` for its dock-entered flag
(`src/sim/radio/receive.rs:82`).

**Step 1: Set the field in the setter.** In `mark_war_factory_spawn_contact`, the success block
currently reads:
```rust
    produced.mark_live_contact_with(producer_id);
    true
```
Change to:
```rust
    produced.mark_live_contact_with(producer_id);
    // gamemd ExitObject_Main also sends 0x18 (sets +0x418) beside the HELLO contact;
    // the footprint-clear break (tick_war_factory_exit_contacts) gates on this flag.
    produced.dock_entered_with = Some(producer_id);
    true
```

**Step 2: Widen the helper visibility** so the sweep module can reuse it. Change:
```rust
fn exact_land_vehicle_exit_factory(rules: &RuleSet, structure_id: &str) -> bool {
```
to:
```rust
pub(super) fn exact_land_vehicle_exit_factory(rules: &RuleSet, structure_id: &str) -> bool {
```

**Step 3: Extend the setter test.** In `war_factory_spawn_contact_is_marked_per_produced_mover`
(`production_tests.rs`), after the existing `has_live_contact_with(10)` assertion (around line 754),
add:
```rust
    assert_eq!(
        sim.substrate.entities.get(produced).unwrap().dock_entered_with,
        Some(10),
        "WF exit must set the dock-entered (+0x418) flag toward the factory"
    );
    assert_eq!(
        sim.substrate.entities.get(unrelated).unwrap().dock_entered_with,
        None,
        "unrelated vehicles get no dock-entered flag"
    );
```

**Step 4: Verify.**
Run: `cargo test -p vera20k war_factory_spawn_contact_is_marked_per_produced_mover -- --nocapture`
Expected: `test result: ok`.

**Step 5: Commit.** `Slice 7d T1: set dock_entered_with (+0x418) at war-factory exit`

---

### Task 2: Create the footprint-clear break sweep + tests

**Why:** Reproduce gamemd's per-cell-process break (`0x0073A93D` → `0x08` → `0x19`/`0x03`) as a
deterministic post-ground-movement sweep.

**Files:**
- Create: `src/sim/production/war_factory_exit.rs`
- Modify: `src/sim/production/mod.rs:15` (module decl) and exports
- Modify: `src/sim/production/production_tests.rs` (3 new tests + imports)

**Pattern:** collect-ids-then-mutate, mirroring `tick_gate_runtimes` (`src/sim/gate_runtime.rs:175`).

**Step 1: Create the sweep module.** Write `src/sim/production/war_factory_exit.rs`:
```rust
//! War-factory exit radio-contact transient break.
//!
//! A newborn land vehicle from a war factory holds a live radio contact with its
//! producer so it can drive across the factory footprint (the NumberImpassableRows
//! row-skip read in `build_live_building_entry_skip_map`). The producer reproduces
//! gamemd by breaking that contact the moment the vehicle's per-cell process finds
//! no building under its current cell (footprint cleared). Despawn / limbo cleanup
//! (`clear_radio_contacts_for`) remains the safety net. sim/ only — depends on
//! map/entities, rules, and sim::{entity_store,intern,movement::locomotor,occupancy}.

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::StringInterner;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;

use super::production_spawn::exact_land_vehicle_exit_factory;

/// Break each war-factory exit contact whose vehicle has cleared the factory
/// footprint. Runs once per tick, right after ground movement.
///
/// Gates (all must hold), matching gamemd's per-cell-process break:
/// - the mover is a vehicle carrying a dock-entered flag (`+0x418`) toward a producer;
/// - that producer is a WeaponsFactory land-vehicle exit factory (the refinery's
///   dock-entered flag points at a refinery → skipped, leaving its lifecycle intact);
/// - the mover's current cell has no `Structure` occupant (footprint cleared).
pub fn tick_war_factory_exit_contacts(
    entities: &mut EntityStore,
    occupancy: &OccupancyGrid,
    rules: &RuleSet,
    interner: &StringInterner,
) {
    // Pass 1 (immutable reads): decide which (mover, producer) contacts to break.
    // `entities.values()` iterates in stable_id order → deterministic.
    let to_break: Vec<(u64, u64)> = {
        let ents: &EntityStore = entities;
        ents.values()
            .filter_map(|mover| {
                if mover.category != EntityCategory::Unit {
                    return None;
                }
                let producer_id = mover.dock_entered_with?;
                let producer = ents.get(producer_id)?;
                if producer.category != EntityCategory::Structure {
                    return None;
                }
                if !exact_land_vehicle_exit_factory(rules, interner.resolve(producer.type_ref)) {
                    return None;
                }
                let on_footprint = occupancy
                    .get(mover.position.rx, mover.position.ry)
                    .is_some_and(|cell| {
                        cell.blockers(MovementLayer::Ground).any(|id| {
                            ents.get(id)
                                .is_some_and(|o| o.category == EntityCategory::Structure)
                        })
                    });
                if on_footprint {
                    return None;
                }
                Some((mover.stable_id, producer_id))
            })
            .collect()
    };

    // Pass 2 (mutable): apply the break (models 0x08 -> 0x19 -> 0x03).
    for (mover_id, producer_id) in to_break {
        if let Some(mover) = entities.get_mut(mover_id) {
            mover.clear_live_contact_with(producer_id);
            mover.dock_entered_with = None;
        }
    }
}
```

**Step 2: Declare + export the module.** In `src/sim/production/mod.rs`, beside the other `mod`
declarations (near line 15) add:
```rust
mod war_factory_exit;
```
and beside the other `pub use self::...` exports add:
```rust
pub use self::war_factory_exit::tick_war_factory_exit_contacts;
```

**Step 3: Add the sweep tests.** In `src/sim/production/production_tests.rs`, add to the imports at
the top (beside the existing `use super::production_spawn::{...}`):
```rust
use super::war_factory_exit::tick_war_factory_exit_contacts;
```
Then add these three tests (they reuse the existing `factory_rules`, `spawn_structure`,
`find_spawn_selection_for_owner`, `Simulation`, `BTreeMap` setup used by
`war_factory_spawn_contact_is_marked_per_produced_mover`):
```rust
#[test]
fn war_factory_exit_contact_held_while_on_footprint() {
    let rules = factory_rules();
    let mut sim = Simulation::new();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    spawn_structure(&mut sim, 10, "Americans", "GAWEAP", 20, 20);
    // Spawn the produced tank ON the factory's occupancy cell. `spawn_structure`
    // registers the test structure at a single occupancy cell (its origin 20,20 —
    // see production_tests.rs:552), so (20,20) is the cell that has a Structure
    // occupant under it. (Real production occupies the full footprint via
    // entity_occupancy_cells; the test helper is the single-cell simplification.)
    let produced = sim
        .spawn_object("MTNK", "Americans", 20, 20, 64, &rules, &height_map)
        .expect("produced tank should spawn");
    assert!(mark_war_factory_spawn_contact(&mut sim, &rules, 10, produced));

    tick_war_factory_exit_contacts(
        &mut sim.substrate.entities,
        &sim.substrate.occupancy,
        &rules,
        &sim.interner,
    );

    let mover = sim.substrate.entities.get(produced).unwrap();
    assert!(
        mover.has_live_contact_with(10),
        "contact must persist while the vehicle is still on the factory footprint"
    );
    assert_eq!(mover.dock_entered_with, Some(10));
}

#[test]
fn war_factory_exit_contact_breaks_when_unit_clears_footprint() {
    let rules = factory_rules();
    let mut sim = Simulation::new();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    spawn_structure(&mut sim, 10, "Americans", "GAWEAP", 20, 20);
    // Spawn the produced tank on a clear cell well away from the foundation.
    let produced = sim
        .spawn_object("MTNK", "Americans", 30, 30, 64, &rules, &height_map)
        .expect("produced tank should spawn");
    assert!(mark_war_factory_spawn_contact(&mut sim, &rules, 10, produced));

    tick_war_factory_exit_contacts(
        &mut sim.substrate.entities,
        &sim.substrate.occupancy,
        &rules,
        &sim.interner,
    );

    let mover = sim.substrate.entities.get(produced).unwrap();
    assert!(
        !mover.has_live_contact_with(10),
        "contact must break once the vehicle has cleared the factory footprint"
    );
    assert_eq!(
        mover.dock_entered_with, None,
        "the dock-entered flag (+0x418) must clear with the contact"
    );
}

#[test]
fn war_factory_exit_break_ignores_non_weapons_factory_producer() {
    // Protects the refinery dock lifecycle: a non-UnitType producer's dock-entered
    // flag must never be broken by this sweep. GAPILE (Factory=InfantryType) stands
    // in for any non-WeaponsFactory-land producer.
    let rules = factory_rules();
    let mut sim = Simulation::new();
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    spawn_structure(&mut sim, 10, "Americans", "GAPILE", 20, 20);
    let mover = sim
        .spawn_object("MTNK", "Americans", 30, 30, 64, &rules, &height_map)
        .expect("mover should spawn");
    // Manually emulate a non-WF dock-entered link (as the refinery bus would set).
    let m = sim.substrate.entities.get_mut(mover).unwrap();
    m.mark_live_contact_with(10);
    m.dock_entered_with = Some(10);

    tick_war_factory_exit_contacts(
        &mut sim.substrate.entities,
        &sim.substrate.occupancy,
        &rules,
        &sim.interner,
    );

    let m = sim.substrate.entities.get(mover).unwrap();
    assert!(
        m.has_live_contact_with(10),
        "non-WeaponsFactory dock-entered links must be left to their own lifecycle"
    );
    assert_eq!(m.dock_entered_with, Some(10));
}
```
> If `factory_rules()` does not define `GAPILE`, use any other non-`Factory=UnitType` structure it
> defines (verify with the `infantry_spawn_uses_foundation_center_cell` test at
> `production_tests.rs:764`, which uses `GAPILE`).

**Step 4: Verify.**
Run: `cargo test -p vera20k war_factory_exit -- --nocapture`
Expected: all three new tests `ok`.

**Step 5: Commit.** `Slice 7d T2: footprint-clear break sweep for war-factory exit contacts`

---

### Task 3: Wire the sweep into `advance_tick`

**Why:** Run the break once per tick right after ground movement, matching gamemd's per-cell-process
timing (contact present during the move into the clear cell; gone next tick).

**Files:**
- Modify: `src/sim/world/mod.rs:1837-1845` (the post-ground-movement `if let Some(rules)` block)

**Pattern:** add beside the existing `tick_gate_runtimes` call in the same `rules` block.

**Step 1: Add the call.** The block currently reads:
```rust
        if let Some(rules) = rules {
            crate::sim::gate_runtime::tick_gate_runtimes(
                &mut self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
                self.binary_frame,
            );
        }
```
Change to:
```rust
        if let Some(rules) = rules {
            crate::sim::gate_runtime::tick_gate_runtimes(
                &mut self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
                self.binary_frame,
            );
            // Slice 7d: break each war-factory exit contact whose vehicle has cleared
            // the factory footprint this tick (gamemd's per-cell-process break).
            crate::sim::production::tick_war_factory_exit_contacts(
                &mut self.substrate.entities,
                &self.substrate.occupancy,
                rules,
                &self.interner,
            );
        }
```

**Step 2: Verify build.**
Run: `cargo check -p vera20k`
Expected: builds clean (no errors).

**Step 3: Commit.** `Slice 7d T3: run war-factory exit-contact break after ground movement`

---

### Task 4: Regression + determinism verification

**Why:** Confirm the reader is unaffected, the refinery lifecycle is intact, and lockstep is
preserved (no unexpected baseline shift).

**Files:** none (verification only; a baseline edit only if Step 3 unexpectedly shifts it).

**Step 1: Reader/passability regression.**
Run: `cargo test -p vera20k empty_tank_bunker_is_passable_occupied_blocks refinery_live_skip_map_opens_bib_east_edge_not_interior refinery_contact_number_rows_opens_first_clear_column_only -- --nocapture`
Expected: all `ok` (the skip-map reader is untouched).

**Step 2: Gate + movement-occupancy suites.**
Run: `cargo test -p vera20k gate_runtime movement_occupancy -- --nocapture`
Expected: `test result: ok`.

**Step 3: Determinism.**
Run: `cargo test -p vera20k determinism_replay slice6 -- --nocapture`
Expected: `test result: ok`, and `SLICE6_BASELINE_HASH` **unchanged** (the slice6 scenario has no
war factory, so the setter/sweep never fire). If `replay_hash_stable_through_slice6` fails with a
new `left:` value, **stop and investigate** — the sweep is touching state it should not; do NOT
silently re-baseline.

**Step 4: Full regression.**
Run: `cargo test -p vera20k`
Expected: read the literal `test result:` line — `0 failed`. (Per project memory, do not report
counts before reading the real output.)

**Step 5: Clippy.**
Run: `cargo clippy -p vera20k`
Expected: no new warnings in the touched files.

**Step 6: Commit** only if a file changed in this task (e.g., a justified baseline edit). Otherwise
no commit — Tasks 1-3 already landed the change.

---

## Sources & References
- **Design doc:** docs/plans/2026-06-03-war-factory-exit-contact-break-design.md
- **Ghidra (verified this session):** `0x00443C60` ExitObject_Main (WeaponsFactory branch),
  `0x0044D880` FUN_0044D880 case 0 (Guard), `0x00739EC0` UnitClass::PerCellProcess (`0x0073A93D`
  break gate), `0x006F4AB0` TechnoClass::Receive_Radio case 8.
- **Ghidra reports:** docs/research/WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md,
  miner/STOCK_REFINERY_RADIO_0X08_GLOBAL_SENDERS_GHIDRA_REPORT.md,
  RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md,
  pathfinding/NUMBER_IMPASSABLE_ROWS_CALLSITE_MATRIX_GHIDRA_REPORT.md.
- **Related code:** src/sim/production/production_spawn.rs:189 (setter),
  src/sim/movement/movement_occupancy.rs:326 (reader), src/sim/gate_runtime.rs:175 (pattern),
  src/sim/occupancy.rs (structure-at-cell), src/sim/entity_store.rs:80 (despawn safety net),
  src/sim/world/world_hash.rs:486 (dock_entered_with fold), src/sim/world/slice6_retask_tests.rs:75
  (baseline).
- **INI:** rulesmd.ini `[GAWEAP]/[NAWEAP]/[YAWEAP]` `WeaponsFactory=yes`, `Factory=UnitType`,
  `ExitCoord=512,256,0` (consumed via `exact_land_vehicle_exit_factory`; no new parsing).
