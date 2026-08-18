# War-Factory Exit Radio-Contact Transient Break (Slice 7d) Design

## Goal
Break the war-factory↔newborn-vehicle radio contact the moment the vehicle clears the
factory footprint (gamemd's per-cell-process behavior), instead of holding it until
despawn (current DRIFT).

## Architecture Context
- **Contact set:** `mark_war_factory_spawn_contact` (`src/sim/production/production_spawn.rs:189`,
  call at `:213`) does `produced.mark_live_contact_with(producer_id)` →
  `radio_contacts.insert(producer)`. One-directional (newborn→producer); the reader only
  needs mover→building, so this is parity-correct. Scoped by `exact_land_vehicle_exit_factory`
  (`Factory=UnitType`, `!naval`, `exit_coord.is_some()`) = stock GAWEAP/NAWEAP/YAWEAP.
- **Sole load-bearing reader:** `build_live_building_entry_skip_map`
  (`src/sim/movement/movement_occupancy.rs:326`) → `has_live_contact_with` →
  the `NumberImpassableRows` row-skip exception (same-building cells x ≥ rx+1).
- **Cleared today:** only `clear_radio_contacts_for` (despawn at `world/mod.rs:1038`,
  crush-death at `movement_tick.rs:1662`, passenger paths). No drive-out break → the DRIFT.
- **`+0x418` analog:** `GameEntity.dock_entered_with: Option<u64>` (`game_entity.rs:283`)
  already models gamemd's `+0x418` dock-entered byte for the refinery miner (set via the
  radio bus, `radio/receive.rs:82`). Hashed at `world_hash.rs:486`. `clear_radio_contacts_for`
  already clears both `radio_contacts` and a dangling `dock_entered_with` (`entity_store.rs:85-90`).
- **Tick placement:** `tick_gate_runtimes` (`world/mod.rs:1838`, right after the ground-movement
  phase `tick_movement_with_grids` at `:1813`) is the established precedent: a per-tick
  maintenance sweep that runs after ground movement and before Phase-2 air/special movement.

## gamemd mechanism (live Ghidra-verified, this session)
Full lifecycle across four functions:
1. `ExitObject_Main @ 0x00443C60` (WeaponsFactory branch): Unlimbo unit → building→unit
   `HELLO(0x02)` (reciprocal `Contacts[]`) → `0x18` (unit `+0x418=1`, propagates to building)
   → building `Queue_Mission(0x10)`.
2. `FUN_0044D880` case 0: unit `Assign_Mission(5=Guard)`; open door; cases 1-2 scatter +
   inject Drive locomotor and command the unit out; case 3 waits on the building's `+0x418`
   clearing.
3. `UnitClass::PerCellProcess @ 0x00739EC0` (send site `0x0073A93D`): when the driving unit
   has `+0x418 != 0`, mission ∉ {7 Enter, 0x10 Unload}, and the **current cell has no building
   under it** (footprint cleared), it sends `0x08` to its first contact (the factory).
   The break is **suppressed while a building is under the current cell**.
4. `TechnoClass::Receive_Radio(0x08) @ 0x006F4AB0` (case 8): factory → unit `0x19`
   (clears both `+0x418`) then `0x03` BREAK (removes factory from the unit's `Contacts[]`).

## Impact Analysis
- **Edits:** `production_spawn.rs` (set `dock_entered_with` at exit); new break function +
  its call in `world/mod.rs` after `tick_gate_runtimes`; `world_hash` baseline (`SLICE6_BASELINE_HASH`).
- **Must not change:** the skip-map reader output for existing cases (tank-bunker / refinery /
  gate passability tests stay bit-identical).
- **Must not disturb:** the refinery miner's `dock_entered_with` lifecycle — guaranteed by the
  WeaponsFactory-producer discriminator (a refinery is not a WeaponsFactory → never matched).
- **Determinism:** `radio_contacts` (and `dock_entered_with`) are hashed; changing WHEN the
  break fires shifts the hash timeline → **one documented `SLICE6_BASELINE_HASH` re-baseline**.
  Sweep iterates the BTreeMap in stable-id order; zero RNG; integer/Option math only.

## Chosen Approach — model `+0x418` faithfully (Approach A)
Reuse the generic gamemd primitives (`radio_contacts` = `Contacts[]`, `dock_entered_with`
= `+0x418`) exactly as gamemd does, consistent with the refinery's `+0x418` modeling.

### Components
1. **Set (WF exit).** In `mark_war_factory_spawn_contact`, alongside the existing
   `mark_live_contact_with(producer_id)`, also set
   `produced.dock_entered_with = Some(producer_id)` — models `0x18 → +0x418`.
2. **Break sweep.** New `tick_war_factory_exit_contacts(entities, occupancy, interner, rules)`,
   a post-ground-movement maintenance step. For each `Unit` entity with
   `dock_entered_with == Some(p)`:
   - Resolve `p`; **require it is a Structure satisfying `exact_land_vehicle_exit_factory`**
     (the WeaponsFactory-producer discriminator; refinery `dock_entered_with` points at a
     refinery → skipped). If `p` no longer resolves to such a structure → no-op (limbo/despawn
     cleanup already handles a sold factory).
   - If the unit's current cell `(rx, ry)` has **no Structure occupant** (footprint cleared) →
     `radio_contacts.remove(p)` **and** `dock_entered_with = None` (models `0x08→0x19→0x03`).
   - Else (still on a footprint cell) → no-op (gamemd suppression while a building is under
     the cell).
3. **Call site.** Insert in `advance_tick` immediately after `tick_gate_runtimes`
   (`world/mod.rs:~1845`), gated on `rules` being `Some` (same as gate runtimes).
4. **Safety net.** `clear_radio_contacts_for` unchanged — despawn/death/limbo still clears
   both fields (models the limbo BREAK).

### Interfaces / Contracts
- Footprint-clear predicate = "no `Structure`-category occupant at the unit's current cell"
  (matches gamemd's `Look_up_building_in_cell == 0`, i.e. *any* building, not just the
  producer). Implement via the occupancy structure-layer lookup; if not ergonomic, fall back
  to the producer's `building_base_foundation_cells` membership (note: producer-only is a
  minor narrowing of the *any*-building gate — flag if used).
- Discriminator = producer is a WeaponsFactory land-vehicle exit factory. This is
  **output-equivalent** to gamemd's `mission ∉ {7, 0x10}` gate for all stock scenarios
  (WF unit = Guard + WF contact → break; refinery miner = Enter/Unload + refinery contact →
  not a WF contact → no break), and avoids depending on the not-yet-authoritative
  `MissionCom` enum (Slice 8).

### Data Flow (timing parity)
- Tick T: ground movement builds the skip map with the contact **present** (unit still paths
  through the footprint), unit drives to the clear cell, position commits. `tick_war_factory_exit_contacts`
  runs same tick T → unit off-footprint with WF `dock_entered_with` → break.
- Tick T+1: skip map built **without** the contact → factory is a normal blocker for that unit.
- Matches gamemd: PerCellProcess break fires during movement T (after the move into the clear
  cell), gone by T+1.

### Error Handling
- Producer despawned/sold: `dock_entered_with` is already nulled by `clear_radio_contacts_for`
  when the factory limbos (`entity_store.rs:85`); the sweep's resolve-and-require-WeaponsFactory
  guard makes a stale pointer a safe no-op.
- Unit never leaves the footprint (killed on exit cell / no move order): contact persists until
  despawn — matches gamemd (PerCellProcess never sees a clear cell); safety net clears it.

### Testing Strategy
- `war_factory_exit_contact_breaks_when_unit_clears_footprint`: spawn GAWEAP, mark contact
  (asserts `dock_entered_with` set); unit on footprint cell → tick → still contacted; move to
  clear cell → tick → `radio_contacts` no longer contains producer AND `dock_entered_with == None`.
- `war_factory_exit_contact_held_while_on_footprint`: unit on a footprint cell → tick → retained.
- `war_factory_exit_break_ignores_refinery_dock_entered_with`: miner with
  `dock_entered_with = Some(refinery)` off the refinery footprint → tick → contact retained
  (discriminator guard; protects the refinery lifecycle).
- `war_factory_exit_contact_set_marks_dock_entered_with`: the setter sets both fields.
- Regression (must stay green, unchanged): `empty_tank_bunker_is_passable_occupied_blocks`,
  `refinery_live_skip_map_opens_bib_east_edge_not_interior`,
  `refinery_contact_number_rows_opens_first_clear_column_only`, the gate tests, `determinism_replay`.
- Re-baseline `SLICE6_BASELINE_HASH` once, one-line documented reason.

## Architectural Decisions
- **Follows** the `tick_gate_runtimes` precedent (post-ground-movement maintenance sweep) for
  placement and shape.
- **Reuses** the `dock_entered_with` (`+0x418`) primitive consistently with the refinery —
  "model the primitive, not approximate it."
- **Avoids** coupling to the in-flux mission enum by using a producer-type discriminator that
  is provably output-equivalent to gamemd's mission gate for stock content.
- **Leaves untouched:** the skip-map reader (no passability-test risk); the one-directional
  contact (parity-correct for the reader); the building-side `+0x418` and the WF door/drive-out
  FSM (Rust does not replicate the door FSM — the only observable consequence of the whole
  `+0x418`/contact dance is the row-skip passability, which this closes).

## Alternatives Considered
- **Approach B (collapse to contact link):** don't set `dock_entered_with`; break by scanning
  `radio_contacts` for a WeaponsFactory producer. Output-identical, simpler, but the WF unit's
  internal state would not mirror gamemd's `+0x418` byte. Rejected (user chose faithful A).
- **In-movement-loop per-cell hook:** put the break at the cell-commit point inside
  `tick_movement_with_grids`. Rejected: multiple commit sites (`movement_step.rs:98`,
  `movement_tick.rs:1280`), borrow-heavy, and output-identical to the centralized post-movement
  sweep for a unit that starts on the footprint (first clear-cell tick = cell-entry tick).
