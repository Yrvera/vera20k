# Generic Despawn / Limbo Cleanup Entry Points - Rust Reconciliation Report

**Address(es):** prior verified binary facts from `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`: `0x0065AA80`, `0x0065ACE0`, `0x0065A970`, `0x0065A820`
**Investigation Mode:** coverage-map
**Claimed Scope:** Rust-source inventory of current entity removal, hide, consume, and dock-contact cleanup entry points, reconciled against the verified gamemd `Broadcast_Radio_ToAll(3)` limbo BREAK cleanup behavior.
**Non-Scope:** fresh binary decompilation, full RadioClass protocol, full CargoClass destructor order, CaptureManager/mind-control internals, implementing the Rust fix.
**Confidence:** High for Rust-source observations; High for binary facts copied from the prior Ghidra report; Medium for which future helper shape is best because no code was changed.
**Active in YR:** Yes for Techno limbo/death cleanup; Rust inventory is current source state.

## 1. Overview

The verified gamemd behavior is centralized at Techno limbo: normal Techno limbo/death broadcasts `BREAK(3)` to every non-null radio contact before `ObjectClass::Conceal` flips limbo state. Rust does not currently have an equivalent central pre-despawn/pre-hide broadcast. `Simulation::despawn_entity` is only one of several removal paths, and even it only removes origin-cell occupancy plus the entity.

Current Rust cleanup is therefore not centralized enough for `GameEntity.radio_contacts`. Dock reservation systems scrub their own maps, but generic `radio_contacts` is state-hashed and can remain on peers unless each producer/removal path clears it explicitly.

## 2. Verified Docs Facts

| Fact | Evidence | Confidence |
|---|---|---|
| Techno limbo tail broadcasts `BREAK(3)` before conceal when the object is not already in limbo. | `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`, `0x0065AA80`, `0x005F4D30` | High |
| `RadioClass::Broadcast_Radio_ToAll(3)` loops all non-null contact slots. | same report, `0x0065ACE0` | High |
| Sender-side BREAK clears matching target slots before target receive dispatch. | same report, `0x0065A970` | High |
| Target-side BREAK clears the matching sender slot after receive side effects. | same report, `0x0065A820` | High |
| Building limbo also reaches the same Techno limbo helper; the `0x007F05DC` xref is a vtable slot, not an extra caller. | same report, `0x00445880`, `0x006F6AC0`, `0x007F05DC` | High |

## 3. Rust Source Observations

### Generic Contact State

| Surface | Source observation | Cleanup status |
|---|---|---|
| `GameEntity.radio_contacts` | Stored per entity as `Vec<u64>` for RadioClass-style live contacts. | Exists, not globally scrubbed. Evidence: `src/sim/game_entity.rs:115-120`. |
| `mark_live_contact_with` / `clear_live_contact_with` | Helpers are idempotent; `clear_live_contact_with` only removes a specified peer ID from one entity. | No production caller of `clear_live_contact_with` was found in this pass. Evidence: `src/sim/game_entity.rs:421-438`, `rg clear_live_contact_with`. |
| State hash | Radio contact count and IDs are hashed. | Stale contacts are deterministic-state/replay relevant. Evidence: `src/sim/world/world_hash.rs:438-441`, test `live_radio_contacts_change_state_hash_per_mover` at `src/sim/world/world_hash.rs:765`. |
| Movement passability | Building entry skip checks `mover.has_live_contact_with(building.stable_id)`. | Stale contact IDs can preserve a contacted-building passability exception. Evidence: `src/sim/movement/movement_occupancy.rs:190`. |
| War factory spawn producer | Produced land vehicles mark contact with the producing factory. | Producer path creates generic contacts. Evidence: `src/sim/production/production_spawn.rs:177-181`, `src/sim/production/production_queue.rs:535-539`. |

### Removal / Limbo-like Entry Points

| Entry point | Source observation | Uses `despawn_entity`? | Generic `radio_contacts` cleanup? |
|---|---|---:|---:|
| `Simulation::despawn_entity` | Removes origin-cell occupancy, decrements owned count for non-dying entities, then removes from `EntityStore`. | It is the helper. | No. Evidence: `src/sim/world/mod.rs:530-555`. |
| MCV deploy to ConYard | Despawns vehicle, then spawns building. | Yes. | No beyond `despawn_entity`. Evidence: `src/sim/world/world_spawn.rs:563-569`. |
| Building undeploy finalization | Finished `building_down` despawns building, then spawns the unit. | Yes. | No beyond `despawn_entity`. Evidence: `src/sim/world/mod.rs:972-989`. |
| Slave miner deploy / undeploy | Removes `slave_bindings`, despawns SMIN/YAREFN, spawns counterpart. | Yes. | No beyond `despawn_entity`. Evidence: `src/sim/slave_miner.rs:466-473`, `src/sim/slave_miner.rs:548-555`. |
| Engineer capture consumption | Ownership changes, then engineer is consumed. | Yes. | No beyond `despawn_entity`. Evidence: `src/sim/world/world_orders.rs:232-243`. |
| Engineer bridge repair consumption | Bridge repair scan/effects run, then engineer is consumed. | Yes. | No beyond `despawn_entity`. Evidence: `src/sim/world/world_orders.rs:352-361`. |
| Building sell | Ejects sell survivors/garrison, interrupts docked miners for refineries, then directly removes the building. | No. | No. Evidence: `src/sim/production/production_sell.rs:558-565`. |
| Combat immediate death | Structures and voxel vehicles remove occupancy and directly remove the entity. | No. | No. Evidence: `src/sim/combat/mod.rs:980-986`. |
| Combat animated death | Infantry/SHP entities are marked `dying=true`; app tick removes them after death animation. | No at final removal. | No. Evidence: `src/sim/combat/mod.rs:957-978`, `src/app_sim_tick.rs:290-299`. |
| Crush kills | Movement tick sets HP to zero and directly removes the victim. | No. | No. Evidence: `src/sim/movement/movement_tick.rs:1039-1052`. |
| Passenger boarding / garrison entry | Passenger is hidden by `PassengerRole::Inside`, movement/attack/order are cleared. | No physical remove. | No. Evidence: `src/sim/passenger.rs:399-405`. |
| Paradrop payload preloading | Spawned passengers are immediately set `Inside`; transient occupancy is removed. | No physical remove. | No. Evidence: `src/sim/superweapon/paradrop.rs:196-204`. |
| Passenger/garrison destruction overflow | If no placement cell exists, passengers are marked dead/dying and unhidden. | No immediate remove. | No. Evidence: `src/sim/production/production_sell.rs:296-303`, `src/sim/production/production_sell.rs:441-449`. |
| Garrison building destruction helper | Removes the building before ejecting destruction garrison occupants. | No. | No. Evidence: `src/sim/passenger.rs:1055-1067`. |
| Aircraft self-destruct / crash-like silent despawn | Aircraft are marked `dying=true`, mission cleared. Final removal is later. | No. | No. Evidence: `src/sim/aircraft/mod.rs:615-620`, `src/sim/aircraft/mod.rs:758-765`. |
| Wall overlay paired entity removal | Wall entity is directly removed when overlay damage destroys the wall. | No. | Probably not applicable to Techno radio contacts. Evidence: `src/sim/world/mod.rs:791-813`. |

## 4. Dock / Contact Cleanup Centralization

| System | Current cleanup | Centralization assessment |
|---|---|---|
| Refinery dock contacts | `RefineryDockContacts::cleanup_dead` retains only alive refineries/miners across `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`; miner tick builds alive from `!e.dying`. | Good for refinery-specific maps, not generic `GameEntity.radio_contacts`. Evidence: `src/sim/miner/miner_dock.rs:31-36`, `src/sim/miner/miner_dock.rs:138-156`, `src/sim/miner/miner_system.rs:115-124`. |
| Repair depot dock reservations | `DockReservations::cleanup_dead` removes dead buildings/miners and can promote queued units; building dock tick excludes `dying` entities from alive. | Good for depot reservation map, not generic `radio_contacts`. Evidence: `src/sim/miner/miner_dock.rs:277-294`, `src/sim/docking/building_dock.rs:73-84`. |
| Airfield docks | `AirfieldDocks::cleanup_dead` drops dead airfields and releases dead aircraft, but `tick_aircraft_docks` uses all current entity IDs as alive. Dying aircraft remain alive to this cleanup until physically removed. | Partial; not a generic BREAK cleanup and may lag for dying aircraft. Evidence: `src/sim/docking/aircraft_dock.rs:237-256`, `src/sim/docking/aircraft_dock.rs:348-352`. |
| Building sell and refinery interruption | Sell calls `interrupt_refinery_docked_miners` before direct removal. | Covers a refinery-specific case, but does not scrub generic `radio_contacts` or other dock/contact abstractions. Evidence: `src/sim/production/production_sell.rs:561-565`. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| gamemd pre-conceal BREAK broadcast | verified-from-doc | `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md` | none for this Rust reconciliation |
| `Simulation::despawn_entity` | verified-by-source | `src/sim/world/mod.rs:530-555` | implement not performed |
| Direct `EntityStore::remove` paths in `src/sim` and app tick | verified-by-source | `rg "entities\\.remove"` plus source reads listed above | future scan should re-run after code changes |
| passenger/garrison hide and consume paths | touched-not-exhausted | `src/sim/passenger.rs`, `src/sim/production/production_sell.rs`, `src/sim/superweapon/paradrop.rs` | full CargoClass binary order out of scope |
| aircraft crash/dying paths | touched-not-exhausted | `src/sim/aircraft/mod.rs`, `src/sim/docking/aircraft_dock.rs` | exact aircraft crash parity out of scope |
| slave miner transform paths | verified-by-source | `src/sim/slave_miner.rs:466-555` | no implementation performed |
| dock cleanup managers | verified-by-source | `src/sim/miner/miner_dock.rs`, `src/sim/docking/building_dock.rs`, `src/sim/docking/aircraft_dock.rs` | no generic radio integration exists |

## 6. Implementation Handoff

| Verified behavior / source observation | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario / proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|
| gamemd Techno limbo broadcasts `BREAK(3)` before conceal and reciprocally clears radio contacts. | `0x0065AA80`, `0x0065ACE0`, `0x0065A970`, `0x0065A820` via prior report | Missing generic equivalent | `src/sim/world/mod.rs`, `src/sim/game_entity.rs`, all removal/hide callers | Add one deterministic pre-despawn/pre-limbo cleanup point that removes the disappearing ID from every peer's `radio_contacts` and clears the entity's own contacts before physical removal or hide. | `despawn_entity_broadcast_break_clears_peer_radio_contacts` | Do not rely on refinery/depot/airfield reservation cleanup; those maps are not the generic hashed `radio_contacts`. |
| Not all removals call `Simulation::despawn_entity`. | source rows in section 3 | A helper inside only `despawn_entity` misses combat immediate death, sell, crush, app death-animation final remove, passenger garrison building removal, and wall direct removal. | `src/sim/combat/mod.rs`, `src/sim/production/production_sell.rs`, `src/sim/movement/movement_tick.rs`, `src/app_sim_tick.rs`, `src/sim/passenger.rs` | Either route Techno-like removals through the same sim cleanup helper or call the helper at each pre-remove/pre-hide site. | `combat_immediate_death_clears_radio_contacts_before_remove`, `sell_building_clears_radio_contacts_before_remove`, `crush_kill_clears_radio_contacts_before_remove` | Do not put the only fix in app-level death animation removal; cleanup must happen when the sim state first becomes dead/limbo-like. |
| Boarding/passenger hide is limbo-like even though the entity remains in `EntityStore`. | `src/sim/passenger.rs:399-405`; prior report notes `CargoClass::AddPassenger` calls passenger limbo in gamemd | Missing for hidden passengers | `src/sim/passenger.rs`, `src/sim/superweapon/paradrop.rs` | Clear generic radio contacts when an entity becomes `PassengerRole::Inside`. | `passenger_boarding_clears_radio_contacts_before_inside_state`, `paradrop_preloaded_passenger_has_no_radio_contacts` | Do not treat "still in EntityStore" as proof no limbo cleanup is needed. |
| Dock managers scrub their own reservation state but not generic contacts. | `src/sim/miner/miner_dock.rs:138-156`, `src/sim/docking/building_dock.rs:73-84`, `src/sim/docking/aircraft_dock.rs:237-256` | Partial cleanup only | `ProductionState.dock_reservations`, `depot_dock_reservations`, `airfield_docks`, `GameEntity.radio_contacts` | Keep specific reservation cleanup, but add generic contact cleanup separately. | `dock_cleanup_dead_does_not_leave_game_entity_radio_contact`, `aircraft_dying_releases_or_clears_contact_before_final_remove` | Do not delete the existing reservation cleanup; it carries FIFO/promotion semantics that generic BREAK cleanup should not replace blindly. |
| War factory production creates a generic `radio_contacts` edge used by passability and hashing. | `src/sim/production/production_spawn.rs:177-181`, `src/sim/movement/movement_occupancy.rs:190`, `src/sim/world/world_hash.rs:438-441` | Stale contact can affect pathing/hash after producer or produced unit disappears | production spawn, movement occupancy, world hash tests | Cleanup must cover both produced unit despawn and producer building removal. | `war_factory_contact_removed_when_vehicle_dies`, `war_factory_contact_removed_when_factory_sold`, `state_hash_changes_back_after_radio_contact_cleanup` | Do not make building contact a global passability flag; current per-mover scope is intentional. |

## 7. Negative Facts / Do Not Do

- Do not assume `Simulation::despawn_entity` is the only removal path; multiple direct `entities.remove` sites bypass it today.
- Do not clear only the removed entity's `radio_contacts`; gamemd BREAK also clears peer-side slots.
- Do not replace refinery/depot/airfield reservation cleanup with generic radio cleanup; those systems own queue and promotion state.
- Do not implement mind-control or passenger cargo ownership release as a RadioClass contact side effect; those are adjacent systems.
- Do not require generic radio cleanup for wall/overlay entities unless a future scan proves they can carry Techno-style radio contacts.

## 8. Remaining Uncertainty

- No fresh Ghidra pass was performed in this slot. Binary facts are inherited from the prior verified `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`, per scope.
- Full CargoClass destructor/unload order remains out of scope; the Rust-facing conclusion is limited to "entering Inside/passenger hide needs limbo-like contact cleanup."
- Aircraft crash parity remains out of scope; source shows dying-state paths and airfield dock cleanup timing, but not whether every aircraft death should release docks at dying-time or final removal-time.
- A future code pass should re-run `rg "entities\\.remove|despawn_entity|PassengerRole::Inside|dying = true"` after edits, because this inventory is source-current as of this report.

## 9. Stale Docs / Follow-up Docs

`RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` section 8.5 still says `Broadcast_Radio_ToAll` has "exactly one call site identified this pass" and discusses uncertainty around `0x007F05DC`. Suggested replacement wording:

> `TechnoClass::Limbo_Tail_CallConceal @ 0x0065AA80` is reached through `TechnoClass::Limbo_Helper @ 0x006F6AC0`, whose direct callers are `FootClass::Limbo @ 0x004DB260` and `BuildingClass::Limbo @ 0x00445880`. The data xref at `0x007F05DC` is `vtable__RadioClass` base `0x007F0508` plus `0xD4`, the virtual Limbo slot pointing to `0x0065AA80`, not an unknown second runtime caller.

No other stale-doc correction was identified in this pass.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/RUST_RADIO_ABSTRACTION_GAP_SCAN_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_entity.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_spawn.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_orders.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_tick.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_occupancy.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/passenger.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_spawn.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_sell.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/docking/building_dock.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/docking/aircraft_dock.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/aircraft/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/slave_miner.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/superweapon/paradrop.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs`
