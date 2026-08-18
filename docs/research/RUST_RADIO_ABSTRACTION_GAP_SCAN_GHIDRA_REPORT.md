# Rust Radio Abstraction Gap Scan - Ghidra Research Report

**Target:** `RUST_RADIO_ABSTRACTION_GAP_SCAN`  
**Scope:** Rust-facing scan of current radio-equivalent state: `DockReservations`, `AirfieldDocks`, passenger/transport, `entity.radio_contacts`, world hash, despawn cleanup, and miner dock sequence.  
**Non-scope:** no fresh Ghidra investigation, no Rust edits, no INI edits, no broad gameplay re-audit.  
**Active in YR:** Yes for the already-documented RadioClass semantics cited below; this report adds Rust-source reconciliation only.

## Working Notes

- **Target question:** Where does current Rust have radio-equivalent state, where is it deliberately per-system rather than generic, and what implementation handoff is safest?
- **Non-goals:** Do not rediscover RadioClass; do not re-open stale refinery binary questions; do not implement Rust.
- **Evidence needed to mark COMPLETE:** cite verified docs facts, cite current Rust source locations, and provide concrete handoff/test options.
- **Stop conditions:** stop at source/docs reconciliation; list unresolved binary/source-timing gaps as uncertainty.

## Summary

Rust does not currently have a generic `RadioClass` core. It has several local approximations:

- `GameEntity.radio_contacts` is a per-mover vector used for live passability exceptions such as war-factory exits.
- `RefineryDockContacts` is a specialized refinery contact model with `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`.
- `DockReservations` and `AirfieldDocks` are reservation/queue managers for depot and airfield-style docking.
- Passenger/cargo uses direct `PassengerRole` and `PassengerCargo` state, not RadioClass contacts.
- Cleanup is piecemeal: reservation managers scrub dead IDs, but `Simulation::despawn_entity` does not broadcast/clear generic radio contacts.

The implementation choice is therefore between:

1. **Explicit generic radio core:** model sparse per-entity contact slots, HELLO/BREAK, capacity, and synchronous receive return codes once, then adapt refinery, production exit, service/depot, airfield, and passenger-like systems around it.
2. **Targeted per-system fixes:** keep local models but add missing generic invariants where player-visible: reciprocal/sparse contacts, despawn BREAK cleanup, capacity from `NumberOfDocks`, and receiver-side return-code effects.

Given current Rust shape, targeted fixes are lower-risk for near-term parity. A generic radio core is attractive only if multiple upcoming systems need shared synchronous message dispatch rather than just contact lifetime/capacity semantics.

## Verified Docs Facts

These are already-verified docs facts used as inputs, not new binary findings:

- `RadioClass` is synchronous RPC: `Transmit_Radio_Impl` calls target `Receive_Radio` directly and returns the callee response on the caller stack; no mailbox or frame-delayed queue is involved. Evidence: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md:31-38`.
- Radio contacts are sparse capacity-bounded slots. Broadcast loops each non-null contact slot, and BREAK nulls sender-side slots before target receive; target-side BREAK then nulls the matching sender slot after base receive side effects. Evidence: `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md:43-45`, `77-107`.
- Normal Techno limbo/destruction broadcasts `BREAK(3)` to all contacts before conceal. Evidence: `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md:33-35`, `146-148`.
- War-factory `NumberImpassableRows` relaxation is per contacted mover, not static building passability. Evidence: `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md:12-14`, `56-69`.
- Stock refinery Mission Enter consumes RadioClass contact slot 0 / target fallback, sends `CAN_DOCK(0x0E)`, and preserves or breaks based on the return and the `+0x418` entered flag. It does not prove a Mission Enter-owned FIFO. Evidence: `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md:33-41`, `67-71`, `94-97`, `121`.

## Rust Source Observations

### Generic contact state

- `GameEntity` has `radio_contacts: Vec<u64>` and helper methods `mark_live_contact_with`, `has_live_contact_with`, and `clear_live_contact_with`. Evidence: `src/sim/game_entity.rs:115-120`, `421-438`.
- The world hash includes contact count and contact IDs, so this state is deterministic and replay-relevant. Evidence: `src/sim/world/world_hash.rs:438-441`, test `live_radio_contacts_change_state_hash_per_mover` at `src/sim/world/world_hash.rs:765-785`.
- Production spawn marks only the produced vehicle with contact to the producer; the observed source call is one-way at the entity level. Evidence: `src/sim/production/production_spawn.rs:177-181`.
- Movement occupancy reads contact from the mover side: `mover.has_live_contact_with(building.stable_id)`. Evidence: `src/sim/movement/movement_occupancy.rs:182-191`.

### Despawn cleanup

- `Simulation::despawn_entity` removes occupancy and removes the entity from `EntityStore`, but it does not broadcast BREAK, clear other entities' `radio_contacts`, or call a contact cleanup helper. Evidence: `src/sim/world/mod.rs:530-555`.
- Some local dock systems clean dead/dying IDs separately: refinery contacts in `tick_miner_system`, depot reservations in `tick_building_docks`, and airfield docks in `tick_aircraft_docks`. Evidence: `src/sim/miner/miner_system.rs:116-124`, `src/sim/docking/building_dock.rs:73-84`, `src/sim/docking/aircraft_dock.rs:349-352`.

### Refinery/miner dock sequence

- `RefineryDockContacts` is the current closest Rust equivalent to RadioClass contacts for refineries. It stores `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`. Evidence: `src/sim/miner/miner_dock.rs:22-37`.
- `hello_or_wait` accepts up to `capacity`, appends accepted miners into `contacts`, and puts excess miners into a Rust FIFO `waiting_retry_queue`. Evidence: `src/sim/miner/miner_dock.rs:42-78`.
- Cleanup scrubs dead refs/miners from refinery contacts, waiting queue, entered map, and pad map. Evidence: `src/sim/miner/miner_dock.rs:138-157`.
- Mission approach sends HELLO-like admission before Mission Enter and keeps rejected miners heading toward `QueueingCell`; Mission Enter re-runs admission, checks `contact_entered`, and only starts the enter handshake if contact/entered and pad state permit. Evidence: `src/sim/miner/miner_dock_sequence.rs:567-648`.
- Depart/interrupt logic uses the refinery contact maps directly, not generic `GameEntity.radio_contacts`. Evidence: `src/sim/miner/miner_dock_sequence.rs:398-440`, `src/sim/miner/miner_dock_sequence.rs:886-889` from source scan.

### Airfield/depot reservations

- `AirfieldDocks` is a multi-slot reservation manager sized by `NumberOfDocks`, with FIFO queues and reverse aircraft-to-pad lookup. Evidence: `src/sim/docking/aircraft_dock.rs:100-116`, `134-166`.
- `AirfieldDocks::release` promotes the next queued aircraft into the just-freed pad. Evidence: `src/sim/docking/aircraft_dock.rs:168-193`.
- `AirfieldDocks::cleanup_dead` drops dead airfields and releases dead aircraft, promoting queued entries. Evidence: `src/sim/docking/aircraft_dock.rs:237-260`.
- Older generic `DockReservations` remains for depot docking, not refinery, and stores one occupant plus FIFO queue. Evidence: `src/sim/miner/miner_dock.rs:202-306`, `src/sim/production/production_types.rs:222-225`.

### Passenger/transport

- Passenger/transport state is direct cargo state (`PassengerRole::{Transport, Boarding, Inside}`), not RadioClass contact state. Evidence: `src/sim/passenger.rs:118-130`.
- Boarding mutates the transport cargo and then hides the passenger as `PassengerRole::Inside`; unloading pops cargo and restores the passenger to map occupancy. Evidence: `src/sim/passenger.rs:334-405`, `441-555`.
- Verified docs say cargo chain cleanup is separate from RadioClass contacts, even though boarding conceal will run the radio BREAK path in gamemd. Evidence: `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md:48`, `148`.

## Implementation Handoff

### Option A - Explicit Generic Radio Core

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Radio contacts are sparse, capacity-bounded, synchronous HELLO/BREAK links. | Add a `sim::radio` core with per-entity sparse slots/capacity, `hello`, `break_contact`, `broadcast_break`, and receiver return-code hooks. | `src/sim/game_entity.rs`, new `src/sim/radio.rs`, production spawn, movement occupancy, refinery/airfield/depot adapters. | A produced vehicle and factory establish reciprocal contact; on despawn of either, both sides lose the link before later path checks. | `radio_break_broadcast_clears_reciprocal_factory_contact_before_path_check` | High migration risk because current refinery/airfield systems encode behavior directly and may not map cleanly to one generic receive table yet. |
| `Broadcast_Radio_ToAll(3)` runs pre-conceal cleanup for all contacts. | Route every sim despawn/limbo path through radio cleanup before entity removal. | `src/sim/world/mod.rs`, combat death removal, production sell/undeploy, paradrop silent despawn. | Unit with factory contact dies; unrelated mover still blocks on factory footprint and the dead unit does not retain stale contact in hash. | `despawn_broadcast_break_removes_stale_radio_contacts_from_all_entities` | Medium-high: must avoid treating passengers/cargo and mind-control as RadioClass-owned. |

### Option B - Targeted Per-System Fixes

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Stale radio contacts must not survive limbo/death and affect contacted-building passability. | Add a small contact cleanup helper that removes `stable_id` from every entity's `radio_contacts` when `despawn_entity` or pre-despawn limbo equivalent runs. | `src/sim/world/mod.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_hash.rs`. | Produced vehicle has contact with `GAWEAP`; after vehicle despawn, state hash and movement passability no longer include that contact. | `despawn_entity_clears_stale_live_radio_contacts_from_other_entities` | Low-medium: fixes current visible hazard without forcing full RadioClass dispatch. |
| Refinery contacts are capacity-bounded by `NumberOfDocks` and Mission Enter is return-code driven, not generic FIFO-driven. | Keep `RefineryDockContacts`, but keep the Rust FIFO documented as a retry policy and preserve/break behavior based on `contact_entered` vs live-refinery refusal. | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`. | Second full miner waits while first is on pad; after contact admission, accepted-cell target is anchor `+(3,1)`, and a non-ROGER before entered breaks/reselects. | `refinery_waiter_retries_contact_without_promoting_fifo_as_radio_slot` | Medium: timing remains approximate unless Mission Enter timer/jitter is modeled. |
| Airfield/helipad docking uses contact capacity/slot selection but current Rust is reservation-only. | Keep `AirfieldDocks` for pads, but add/verify radio-equivalent contact lifetime and cleanup only where aircraft passability/reload behavior depends on it. | `src/sim/docking/aircraft_dock.rs`, `src/sim/aircraft/mod.rs`. | Four-pad airfield admits four aircraft by pad index; destroyed aircraft/airfield clears pad and any contact-equivalent state without leaving a stuck reload queue. | `airfield_destroyed_aircraft_releases_pad_and_contact_equivalent_state` | Medium: aircraft-specific `Receive_Radio` gates and cached dock behavior are documented elsewhere and should not be folded in blindly. |

## Negative Facts / Do Not Do

- Do not add a queued radio-message mailbox. RadioClass is synchronous and returns response codes directly. Evidence: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md:31-38`.
- Do not model `NumberImpassableRows` as static pathgrid unblocking for everyone. It is per contacted mover. Evidence: `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md:12-14`, `56-69`.
- Do not treat cargo/passenger chains as RadioClass contact arrays. Cargo uses separate cargo head/count state; RadioClass BREAK cleanup is adjacent but not the cargo storage model. Evidence: `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md:48`, `148`; Rust `src/sim/passenger.rs:118-130`.
- Do not treat Rust `waiting_retry_queue` as a verified gamemd Mission Enter FIFO. Current verified Mission Enter report did not find a FIFO in `0x004D9290`. Evidence: `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md:67-71`, `97`, `121`.
- Do not clear refinery dock/target state solely because one post-enter `CAN_DOCK` is non-ROGER; the entered flag changes the abort behavior. Evidence: `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md:33-41`, `105-107`.

## Remaining Uncertainty

- Whether to invest in a generic radio core depends on upcoming scope: if service depot, airfield, carryall, passenger transport, bunker, and grinder all need synchronous `Receive_Radio` return-code behavior soon, a generic core may pay for itself; if the near-term risks are stale contacts and refinery waiting, targeted fixes are safer.
- Current Rust `GameEntity.radio_contacts` is mover-side and one-way for production spawn; gamemd contacts are reciprocal. For the current war-factory passability use this may be sufficient, but a generic radio core would need reciprocal sparse slots.
- `AirfieldDocks` capacity/pad behavior was source-scanned only. This report did not reconcile the full aircraft `Receive_Radio` gate/cached dock docs into airfield code.
- Despawn/limbo entry points are numerous (`combat`, sell, undeploy, paradrop silent despawn, engineer consumption, slave miner paths). A cleanup helper must be applied consistently or centralized.
- Existing YELLOW-audited refinery docs should remain subordinate to newer focused reports and trace corrections; this scan did not perform a fresh verify-doc pass.

## Stale Docs / Replacement Wording

No new stale-doc wording was proven in this source/docs reconciliation. Use the existing `AUDIT_LOG.md` YELLOW entries for refinery-radio documents as the stale-doc index; do not treat older HARVESTER/RADIO_LINK wording as stronger than the newer focused Mission Enter and two-miner trace reports.

## Status

COMPLETE - source/docs reconciliation only; no Rust edits and no fresh Ghidra investigation performed.
