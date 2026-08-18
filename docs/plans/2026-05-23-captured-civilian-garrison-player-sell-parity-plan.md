# Captured Civilian Garrison Player Sell Parity - Implementation Plan

> Execute this plan task-by-task. Each task is grounded in the approved design and keeps scope to player sell only.

**Goal:** When a player sells a captured civilian `CanBeOccupied=yes` garrison, eject occupants through the existing sell ejection helper and then remove/sell the building normally. Do not preserve/revert the structure from the player-sell command.

**Architecture:** Keep the change inside `src/sim/production/production_sell.rs`. The app command and world command ownership gates are already correct for this scope. `passenger.rs` ownership transfer/revert remains unchanged until the separate civilian reconciliation design.

**Design Doc:** [docs/plans/2026-05-23-captured-civilian-garrison-player-sell-parity-design.md](2026-05-23-captured-civilian-garrison-player-sell-parity-design.md)

**Supersedes:** the captured-civilian player-sell branch in [docs/plans/2026-05-04-unified-garrison-eject-plan.md](2026-05-04-unified-garrison-eject-plan.md). That older plan followed stale evidence and should not be used for this target.

---

## Grounding Summary

- **Verified YR behavior:** after ownership reconciliation makes a civilian garrison player-owned, sell mode treats it as a normal player-owned building target. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` sections 3.1-3.3.
- **Verified sell outcome:** `BuildingClass::Sell` calls `SellBuilding` as an occupant ejection stage, then continues into final sell/removal/refund logic. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` section 3.4.
- **Verified helper contract:** native `SellBuilding @ 0x00457DE0` ejects/clears occupants and does not call `ChangeOwner`. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` sections 3.4 and 8; `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` section 3.5.
- **Current Rust mismatch:** `sell_building` branches on `garrison_original_owner.is_some()`, ejects occupants, emits `StructureAbandoned`, reverts owner, keeps the building, and returns before refund/removal.
- **Scope boundary:** do not change the current immediate transfer/revert model in `passenger.rs`; it is a separate queued reconciliation fix.

## Key Technical Decisions

- **Delete the player-sell captured branch.** `garrison_original_owner` must not decide player-sell outcome.
- **Make the player-sell garrison ejection helper ejection-only.** It should clear cargo but not revert owner.
- **Do not add a mode parameter unless implementation requires it.** `eject_garrison_occupants` is currently used only by player sell; destruction already uses `eject_destruction_garrison`.
- **Keep existing refund/remove side effects.** SpySat reshroud, superweapon refresh, docked-miner interruption, radio contact clear, and credit deposit stay in the normal sell path.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/production/production_sell.rs` | Remove stale captured-civilian player-sell branch, make garrison sell helper ejection-only, add regression tests |

## Interface Changes

None. `production::sell_building(sim, rules, stable_id) -> bool` stays unchanged. No command, app, world, render, UI, audio, or rules API changes.

## Sim Checklist

- [x] No new sim state.
- [x] No new dependency outside `sim/`.
- [x] No new RNG draws.
- [x] No fixed-point or floating-point math changes.
- [x] No `EntityStore` iteration change.
- [x] No deterministic hash format change.

## Risk Areas

- **Helper side effect:** if `eject_garrison_occupants` continues taking `garrison_original_owner`, player sell may still transiently run a native-invalid owner revert before removal.
- **Abandoned event:** player sell must not emit `SimSoundEvent::StructureAbandoned`; that event belongs to empty-garrison reconciliation/unload.
- **Stale comments:** comments currently describe the wrong preserve/revert behavior and must be updated with the verified native split.
- **Over-expansion:** do not fix Scatter mission order, transport LIFO, or civilian reconciliation timing in this plan.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Captured civilian garrison sell removes building | Player-visible command outcome currently wrong | New regression test |
| 1 | Occupants eject alive before removal when exit cells exist | Native `SellBuilding` is still called as ejection stage | New regression test plus existing ejection checks |
| 1 | Player sell does not emit `StructureAbandoned` | Abandon cue belongs to reconciliation, not sell | New regression test |
| 1 | Current owner receives normal refund | Native continues into final sell path | New regression test with nonzero test cost |
| 2 | Ejection helper does not change owner | Native `SellBuilding` does not call `ChangeOwner` | Direct helper-level regression test |
| 3 | Empty captured garrison still sells/removes | Native skips ejection if count zero, then sells | Existing normal path or optional small test |

---

## Tasks

### Task 1: Add failing player-sell regression coverage

**Why:** Pin the verified player-visible behavior before changing the branch.

**File:**
- Modify: `src/sim/production/production_sell.rs`

**Steps:**

1. In the existing `#[cfg(test)] mod tests`, extend or reuse `garrison_edge_rules()`.

   To verify refund, set the test building cost to a nonzero value:

   ```ini
   [CAGAS01]
   Cost=400
   Strength=400
   Foundation=2x2
   CanBeOccupied=yes
   MaxNumberOccupants=5
   ```

   Existing destruction ejection tests do not depend on the cost being zero.

2. Add a helper in the test module to insert a captured civilian garrison sell state:

   ```rust
   fn insert_captured_player_owned_garrison(
       sim: &mut Simulation,
       building_id: u64,
       passenger_id: u64,
   ) {
       let americans = sim.interner.intern("Americans");
       let neutral = sim.interner.intern("Neutral");

       let mut building = GameEntity::test_default(building_id, "CAGAS01", "Americans", 10, 10);
       building.category = EntityCategory::Structure;
       building.owner = americans;
       building.garrison_original_owner = Some(neutral);
       building.passenger_role = PassengerRole::Transport {
           cargo: crate::sim::passenger::PassengerCargo::new(5, 1),
       };
       if let Some(cargo) = building.passenger_role.cargo_mut() {
           assert!(cargo.board(passenger_id, 1));
       }
       sim.entities.insert(building);

       insert_hidden_passenger(sim, passenger_id, building_id, "Americans");
   }
   ```

   If the exact helper needs imports, prefer fully-qualified paths inside the test module instead of broad new production imports.

3. Add test `captured_civilian_garrison_player_sell_removes_building_and_refunds`.

   Shape:

   ```rust
   #[test]
   fn captured_civilian_garrison_player_sell_removes_building_and_refunds() {
       let rules = garrison_edge_rules();
       let mut sim = Simulation::new();
       let building_id = 10;
       let passenger_id = 11;
       insert_captured_player_owned_garrison(&mut sim, building_id, passenger_id);

       let before = credits_for_owner(&sim, "Americans");

       assert!(sell_building(&mut sim, &rules, building_id));

       assert!(sim.entities.get(building_id).is_none());
       assert_eq!(credits_for_owner(&sim, "Americans") - before, 200);

       let passenger = sim.entities.get(passenger_id).expect("passenger should survive sell eject");
       assert!(matches!(passenger.passenger_role, PassengerRole::None));
       assert!(!passenger.dying);
       assert!(
           passenger.position.rx < 10
               || passenger.position.rx > 11
               || passenger.position.ry < 10
               || passenger.position.ry > 11,
           "passenger should be ejected outside the 2x2 foundation"
       );

       assert!(
           !sim.sound_events.iter().any(|event| {
               matches!(
                   event,
                   crate::sim::world::SimSoundEvent::StructureAbandoned { .. }
               )
           }),
           "player sell must not emit StructureAbandoned"
       );
   }
   ```

4. Run the focused test and confirm it fails before implementation:

   ```powershell
   cargo test --lib captured_civilian_garrison_player_sell_removes_building_and_refunds
   ```

   Expected pre-fix failure: building remains, refund is zero, or `StructureAbandoned` is emitted.

### Task 2: Make player-sell garrison ejection ejection-only

**Why:** Native `SellBuilding` ejects/clears occupants but does not change building owner.

**File:**
- Modify: `src/sim/production/production_sell.rs`

**Steps:**

1. In `eject_garrison_occupants`, keep:
   - occupant snapshot;
   - `eject_garrison_passengers_at_edges`;
   - cargo clear;
   - `total_size = 0`;
   - `garrison_fire_index = 0`.

2. Remove the owner revert block:

   ```rust
   if let Some(orig) = building.garrison_original_owner.take() {
       building.owner = orig;
   }
   ```

3. Do not clear `garrison_original_owner` here unless required by compiler/tests. The building is removed by player sell after the helper returns, and preserving the field during the same transaction avoids pretending this helper owns reconciliation state. If borrow or future test needs it cleared before removal, document that it is cleanup-only and not a native owner revert.

4. Update the helper comment from "revert garrison ownership" to "clear player-sell cargo; ownership outcome is handled by the caller/reconciliation path."

5. Add a direct helper-level test `sellbuilding_helper_ejects_without_owner_revert`.

   This test is required because the public `sell_building` regression removes the building after ejection; it cannot observe whether the helper performed a transient native-invalid owner revert before removal.

   Shape:

   ```rust
   #[test]
   fn sellbuilding_helper_ejects_without_owner_revert() {
       let rules = garrison_edge_rules();
       let mut sim = Simulation::new();
       let building_id = 20;
       let passenger_id = 21;
       insert_captured_player_owned_garrison(&mut sim, building_id, passenger_id);

       let americans = sim.interner.intern("Americans");
       let neutral = sim.interner.intern("Neutral");

       assert_eq!(eject_garrison_occupants(&mut sim, &rules, building_id), 1);

       let building = sim
           .entities
           .get(building_id)
           .expect("helper should not remove building");
       assert_eq!(
           building.owner, americans,
           "SellBuilding-style helper must not ChangeOwner"
       );
       assert_eq!(
           building.garrison_original_owner,
           Some(neutral),
           "helper must not consume reconciliation state during player-sell ejection"
       );
       assert!(
           building
               .passenger_role
               .cargo()
               .is_some_and(|cargo| cargo.is_empty()),
           "helper should clear building cargo"
       );

       let passenger = sim.entities.get(passenger_id).expect("passenger should remain");
       assert!(matches!(passenger.passenger_role, PassengerRole::None));
       assert!(!passenger.dying);
   }
   ```

6. Run this focused helper test before and after implementation:

   ```powershell
   cargo test --lib sellbuilding_helper_ejects_without_owner_revert
   ```

   Expected pre-fix failure: building owner becomes `Neutral` and/or `garrison_original_owner` is consumed.

### Task 3: Remove the captured-civilian early return from `sell_building`

**Why:** Native player sell has no preserve/revert branch for captured civilian garrisons.

**File:**
- Modify: `src/sim/production/production_sell.rs`

**Steps:**

1. Update the `sell_building` doc comment. Replace the captured-civilian preserve text with:

   ```rust
   /// Captured civilian `CanBeOccupied` garrisons use the same player-sell
   /// transaction once they are owned by the seller: occupants eject through
   /// the SellBuilding-style helper, then the building is removed/refunded.
   /// Revert-to-civilian belongs to empty-garrison reconciliation, not player sell.
   ```

2. Shrink the snapshot tuple. Remove:
   - `is_captured`;
   - `abandoning_owner`;
   - `entity.garrison_original_owner.is_some()`;
   - `entity.owner` only if it is no longer otherwise used in that tuple.

   Keep:

   ```rust
   let (owner_name, type_id, position, health) = { ... };
   ```

3. Delete the entire early return block:

   ```rust
   if is_captured {
       ...
       return true;
   }
   ```

4. Keep the existing normal path order:
   - refund calculation;
   - `eject_sell_survivors`;
   - `eject_garrison_occupants`;
   - `interrupt_refinery_docked_miners`;
   - `clear_radio_contacts_for`;
   - `entities.remove`;
   - SpySat/superweapon side effects;
   - credit deposit.

5. Do not touch `app_commands.rs`, `world_commands.rs`, or `passenger.rs` for this plan.

### Task 4: Run focused verification

**Why:** This change sits in a central production function and must prove the scoped fix did not disturb existing garrison/abandon behavior.

**Commands:**

```powershell
cargo fmt --check
cargo test --lib captured_civilian_garrison_player_sell_removes_building_and_refunds
cargo test --lib sellbuilding_helper_ejects_without_owner_revert
cargo test --lib production_sell
cargo test --lib garrison
cargo check --lib
```

If `cargo fmt --check` fails due to the new edit, run:

```powershell
cargo fmt
cargo fmt --check
```

**Expected results:**

- New captured civilian player-sell test passes.
- New helper-level no-owner-revert test passes.
- Existing destruction garrison ejection test still passes.
- Existing passenger last-occupant `StructureAbandoned` test still passes.
- `cargo check --lib` passes, allowing existing unrelated warnings.

### Task 5: Review for forbidden scope creep

Before finalizing implementation, inspect the diff and confirm:

- no changes to `src/sim/passenger.rs`;
- no changes to app/UI command gating;
- no Scatter mission implementation;
- no transport cargo order changes;
- no bunker/gate changes;
- no new public API.

Run:

```powershell
git diff -- src/sim/production/production_sell.rs
```

The diff should be limited to test helpers/tests, helper comments/owner-revert removal, and `sell_building` branch removal.
