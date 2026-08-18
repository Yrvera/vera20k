# Garrison Owner Live Object Order Postfix Trace

Date: 2026-05-27

Scenario: two object updates in one tick around one civilian `CanBeOccupied` building:

- A: the infantry entry update runs before the target building update.
- B: the target building update runs before the infantry entry update.

Scope is only civilian garrison owner transfer before/after first occupant entry through Rust's `live_object_order_snapshot` path. Sell, destruction, empty revert, red-health ejection, garrison fire, pips, render, and exact retail map object indices are out of scope.

## Verdict Summary

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Overall status: COMPLETE for the scoped relative-order mechanism. No player-visible FAIL/NOT-IMPLEMENTED finding was found in this concrete trace. The remaining UNCHECKED item is concrete retail object-vector index equality for a specific map/replay instance; that requires runtime logging and is not needed to verify the A/B mechanism.

## Evidence Sources

- Native active-YR scheduler and garrison timing: `docs/research/CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`.
- Native transfer/revert details: `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`.
- Read-only Ghidra spot checks in this run:
  - `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`
  - `BuildingClass::AddGarrisonOccupant @ 0x00522910`
  - `BuildingClass::Update @ 0x0043FB20`
  - `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200`
- Rust read-only surfaces:
  - `src/sim/passenger.rs:266-304`
  - `src/sim/passenger.rs:440-490`
  - `src/sim/passenger.rs:1181-1255`
  - `src/sim/world/mod.rs:575-602`
  - `src/sim/world/world_spawn.rs:253-255`, `426-428`

## Active YR Confirmation

This is active in standard Yuri's Revenge, not dormant TS legacy:

- `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls main objects through `vtable+0x5C` in increasing object-vector index order and reloads the live count after each call.
- `BuildingClass::Update @ 0x0043FB20` is the building `vtable+0x5C` update body.
- `BuildingClass::Update` calls `CheckAutoSellOrCivilian` only when `BuildingType+0x157B` / `CanBeOccupied` is set.
- `BuildingClass::AddGarrisonOccupant @ 0x00522910` appends/limbos the infantry but does not call `ChangeOwner`.
- `CheckAutoSellOrCivilian @ 0x00458200` changes owner when occupant count is positive and current owner is the resolved civilian house.
- Stock YR data contains the scoped objects: `[CAGAS01]` has `CanBeOccupied=yes` and `MaxNumberOccupants=10` at `ini/rulesmd.ini:19302`, `19322-19323`; `[E1]` has `Occupier=yes` at `ini/rulesmd.ini:3713`, `3720`.

## Pipeline

Native:

`LogicClass` live object-vector pass -> infantry `vtable+0x5C` entry path -> `AddGarrisonOccupant` mutates occupant vector only -> later target building `vtable+0x5C` -> `BuildingClass::Update` -> `CheckAutoSellOrCivilian` -> `ChangeOwner(first_occupant.owner, 0)`.

Rust:

`Simulation::advance_tick` phase 6 -> `passenger::tick_passenger_system` -> `Simulation::live_object_order_snapshot()` -> for each ID, process boarding if the ID is a boarding passenger, process garrison unload if applicable, then reconcile that same ID if it is a civilian garrison building.

## Stage Results

### Stage 1 - Active data gates

Native input: CAGAS01 is a stock civilian garrisonable structure and E1 is stock occupying infantry.

Native output: `CanBeOccupied=1`, `MaxNumberOccupants=10`, `Occupier=1`.

Rust input/output: rules parsing exposes `obj.can_be_occupied` and `obj.max_number_occupants`; `reconcile_civilian_garrison_owner_for_building` gates on `obj.can_be_occupied` at `src/sim/passenger.rs:459-464`.

Verdict: PASS. The scoped data gates are present and numerically equal for this scenario: `CanBeOccupied=1`, `MaxNumberOccupants=10`, `Occupier=1`.

### Stage 2 - Boarding does not transfer owner

Native computation: `AddGarrisonOccupant` appends to the building occupant vector and limbos the infantry; no `ChangeOwner` call occurs in that function.

Rust computation: `process_boarding_passenger` is run at `src/sim/passenger.rs:290-296`; the ownership write is not in the boarding call. The focused test `garrison_owner_not_changed_during_boarding_call` asserts changed `false`, owner remains `Neutral`, passenger is `Inside`, and no `garrison_original_owner` is written at `src/sim/passenger.rs:1181-1199`.

Verdict: PASS. Occupant count changes from 0 to 1 and owner changes by 0 in both implementations at boarding commit.

### Stage 3 - Order A: infantry entry before building update

Native computation: the live object vector calls infantry first. Entry appends the occupant, leaving owner unchanged. The later target building update calls `CheckAutoSellOrCivilian`, reads count `1`, sees owner equal to the civilian house, reads first occupant slot `0`, and calls `ChangeOwner(first_occupant.owner, 0)`.

Native output: owner transfer delay after entry commit is `0` global frames for this relative order.

Rust computation: with `live_object_order = [pax, bldg]`, `live_object_order_snapshot()` returns `[pax, bldg]`; `tick_passenger_system` processes the passenger first, then reconciles the building. The test `production_garrison_owner_order_uses_live_object_order_not_stable_id` sets this order at `src/sim/passenger.rs:1226`, calls production `tick_passenger_system` at `1228`, and asserts `changed == true` plus owner `Americans` at `1230-1234`.

Rust output: owner transfer delay after entry commit is `0` passenger-system passes for this relative order.

Verdict: PASS. Literal scoped delay: native `0`, Rust `0`; final owner: infantry owner in both.

### Stage 4 - Order B: building update before infantry entry

Native computation: the live object vector calls the building first. `CheckAutoSellOrCivilian` sees occupant count `0` and does not change owner. The infantry later appends the occupant, but owner transfer waits until the target building's next update.

Native output: owner transfer delay after entry commit is `1` target-building reconciliation pass, normally frame `T+1`.

Rust computation: with order `[bldg, pax]`, the first helper pass reconciles the building before boarding, leaving owner `Neutral`; boarding happens later in that same pass. The next helper pass sees count `1` and changes owner. The test `garrison_owner_waits_next_frame_when_building_update_before_entry` asserts this sequence at `src/sim/passenger.rs:1238-1255`.

Rust output: owner transfer delay after entry commit is `1` passenger-system pass for this relative order.

Verdict: PASS. Literal scoped delay: native `1`, Rust `1`; final owner after the next reconciliation pass: infantry owner in both.

### Stage 5 - Production order source for this mechanic

Native computation: same-frame versus next-frame transfer is selected by the current live `LogicClass` object-vector order.

Rust computation: production `tick_passenger_system` now calls `sim.live_object_order_snapshot()` at `src/sim/passenger.rs:266-268`. The snapshot emits registered live-object IDs first and appends unregistered entity IDs in sorted stable-ID order at `src/sim/world/mod.rs:585-602`. Map and normal spawn paths register live objects at `src/sim/world/world_spawn.rs:253-255` and `426-428`.

Verdict: PASS for the concrete pre-existing registered-object A/B scenarios above. This fixes the prior sorted-stable-ID source drift for the tested garrison owner transfer path.

### Stage 6 - Concrete retail object-vector index equality

Native value needed: actual `LogicClass` vector indices for a selected retail map/replay infantry and target CAGAS01 pair at the tick where entry occurs.

Rust value needed: actual `live_object_order_snapshot()` indices for the same loaded state.

Verdict: UNCHECKED. The mechanism now takes a live-order surrogate instead of stable-ID order, but this trace did not attach a runtime debugger/logger to compute concrete gamemd vector indices for a real map instance and compare them against Rust load/registration order. Do not count a specific map instance as PASS until both index numbers are logged.

## Failures

None in the scoped trace.

## Not Implemented

None in the scoped trace.

## Adjacent Findings

- `live_object_order_snapshot()` is a snapshot, while gamemd reloads the live vector count after each object update. Insert/remove effects during the same scheduler pass remain broader scheduler parity work, not part of this two-pre-existing-object trace.
- Some direct test-only or non-world spawn insertion sites bypass `register_live_object`; the snapshot falls back to sorted stable IDs for unregistered entities. That is not a FAIL for the scoped registered map/production spawn path, but it is a surface to audit before claiming global scheduler parity.
- Empty revert, StructureAbandoned ordering, sell/destruction ejection, red-health ejection, and garrison shot rendering were intentionally not traced here.

## Status

COMPLETE for civilian garrison owner transfer under the two scoped live-object relative orders after the postfix.
