# Mission Deploy Building Refinery Rediscovery Plan

Date: 2026-05-26
Status: READY_FOR_IMPLEMENTATION
Contract: `docs/contracts/2026-05-26-mission-deploy-building-refinery-rediscovery-implementation-contract.md`
Scope: Rust implementation plan only; no code changes in this document.

## Goal

Replace `reserved_refinery` as the state 3/4 unload authority with the stock `Mission_Deploy_Building` adjacent-cell lookup: miner current cell plus `(-1,0)`, then first building in that cell's object list.

`reserved_refinery` remains dock admission/contact bookkeeping. It must not decide credit owner, deposit event building, missing-refinery cargo drain, or state-4 building-dependent checks.

## Current Code Touchpoints

- `src/sim/miner/miner_dock_sequence.rs`
  - `handle_dock_sequence`: currently extracts `ref_sid` from `snap.miner.reserved_refinery` and passes it through all dock phases.
  - `phase_unloading`: currently credits `sim.entities.get(ref_sid)`, emits `BaleDepositEvent { building_id: ref_sid }`, and sets `home_refinery = Some(ref_sid)`.
  - `phase_departing`: currently releases reservation/contact using `ref_sid`, clears display override, and resumes `SearchOre`.
  - `abort_invalid_refinery`: existing missing-refinery cleanup path must not be blindly reused if it clears the unload display in the state-3 null branch.
- `src/sim/occupancy.rs`
  - `OccupancyGrid::get(rx, ry)` exposes cell occupants in deterministic object-list order.
  - Structures are appended to occupied cell lists via `CellListInsertion::AppendBuilding`.
- `src/sim/game_entity.rs`
  - `GameEntity.category == EntityCategory::Structure` identifies building-like entities.
  - `occupancy_list_layer()` gives the gamemd-style ground/bridge object-list selector.

## Implementation Shape

### 1. Add The Lookup Helper

Add a private helper in `miner_dock_sequence.rs` near the dock utility helpers:

```text
labeled behavior:
  mission_deploy_unload_refinery(sim, miner_id) -> Option<u64>

mechanism:
  - read miner entity;
  - if miner current rx is 0, return None;
  - lookup cell = (miner.position.rx - 1, miner.position.ry);
  - choose the object-list layer from the miner's occupancy list layer, falling back to Ground only if needed;
  - iterate sim.occupancy.get(lookup_rx, lookup_ry).iter_layer(layer);
  - return the first occupant whose entity category is Structure.
```

Do not check `reserved_refinery`, `DockingOffset`, `QueueingCell`, `GetDockCoord`, or `+0x2E4` equivalents in this helper.

Keep the helper generic: it mirrors `Look_up_building_in_cell`, so it returns the
first building-like object, not "the first refinery." Callers decide whether
`Refinery=yes` matters. In particular, state 4 has a `Refinery=yes` guard before
the slot-8 wait. Do not bake that guard into the helper.

### 2. Use It In State 3 Dump Gate

In `phase_unloading`, after the timer gate passes and before cargo is removed:

- call the new helper;
- if it returns `Some(building_id)`:
  - use the returned building as the gamemd lookup result;
  - use that building's owner for credits;
  - use that building ID for purifier context and `BaleDepositEvent`;
  - drain cargo exactly where the current code drains cargo;
  - keep `ref_sid` only for eventual reservation/contact release.
- if it returns `None`:
  - do not remove cargo;
  - do not credit anyone;
  - do not emit `BaleDepositEvent`;
  - leave the miner in the Harvest/return equivalent without treating it as emptied.

For a full miner, the null branch must make it eligible to select/return to a
refinery again, not send it to ore search as if cargo were empty. For partial
cargo, follow the existing Harvest state-0 behavior after the mission handoff;
do not invent an immediate new-refinery selection unless the full-cargo gate
would do that.

Important: the current fallback to miner owner when `sim.entities.get(ref_sid)` is missing is not parity. It must not survive in the state-3 dump path.

The state-3 use of a generic building result is intentional. The verified state-3
lookup uses `Look_up_building_in_cell` and the available reports do not show a
state-3 `Refinery=yes` guard before credit/anim work. Do not silently add a
refinery-only filter in state 3; if a future live check finds such a guard, amend
the contract first.

### 3. Model The Null-Lookup Branch Conservatively

For the first patch, keep the stock-visible guarantees:

- preserve cargo;
- leave full miners eligible to select/return to another refinery;
- do not immediately clear the unloading display latch in this branch;
- release the Rust reservation/contact for `ref_sid` enough to avoid stale queues;
- do not route through a broad cleanup helper if it clears the unload display latch in this branch.

The exact same-frame rendered duration after null lookup is still blocked on runtime capture, so the patch should not claim pixel-perfect cleanup there. It should avoid making the known static mismatch worse by clearing the display immediately inside the null branch.

### 4. Use Rediscovery In State 4 Where Building-Dependent

In `phase_departing`, split two concepts:

- `ref_sid`: reservation/contact bookkeeping to release.
- `rediscovered_refinery`: state-4 building-dependent identity.

State 4 behavior should be expressed as:

- rediscover the west-cell building;
- if that building exists, has `Refinery=yes`, and the slot-8/ProductionAnim wait
  is live, remain in the state-4 wait;
- otherwise perform the unload-active cleanup and Harvest scheduling handoff.

For stock `GAREFN/NAREFN`, slot-8 wait is normally absent, so this likely has
little visible effect today. Still structure the code so future
slot-8/ProductionAnim parity reads the west-cell building, not
`reserved_refinery`.

Do not introduce new slot-8 wait machinery in this patch unless a matching
surface already exists. The narrow improvement is the lookup boundary and
authority split; full modded `ProductionAnim` wait parity can remain a later
patch.

## Test Plan

Add focused tests in `src/sim/miner/miner_tests.rs`.

1. `unload_state3_uses_west_cell_building_not_reserved_refinery`
   - Setup miner unloading at accepted pad; `reserved_refinery=A`; place another structure/refinery `B` so `sim.occupancy` contains `B` in the west cell.
   - Run one ready dump gate.
   - Assert credits and `BaleDepositEvent.building_id` use `B`, not `A`.

2. `missing_west_cell_building_does_not_credit_or_emit_deposit_event`
   - Setup miner unloading with cargo; `reserved_refinery` still points to a live or stale refinery elsewhere; no building in west cell.
   - Run one ready dump gate.
   - Assert cargo unchanged, no refinery-owner credits, no miner-owner fallback credits, no purifier bonus, no `BaleDepositEvent`.

3. `state3_null_lookup_preserves_full_cargo_and_returns_to_refinery_selection`
   - Setup full miner mid-unload; remove west-cell building.
   - Run null branch and enough ticks for the current Harvest/return equivalent.
   - Assert cargo remains and miner is eligible for or enters the return/refinery-selection flow; do not require same-tick new refinery reservation.

4. `state3_null_lookup_does_not_clear_unload_display_latch`
   - Setup active `display_type_override` and missing west-cell building.
   - Run null branch only.
   - Assert the branch itself does not clear the override.

5. `reserved_refinery_released_but_not_used_for_unload_credit_identity`
   - Setup reservation/contact for `A`, but west-cell building `B` is present in `sim.occupancy`.
   - Complete unload/depart.
   - Assert reservation/contact for `A` is cleaned up and unload effects used `B`.

6. `state4_refinery_yes_guard_is_caller_owned`
   - Setup state-4 equivalent with a non-refinery structure in the west cell.
   - Run the state-4 handoff.
   - Assert the lookup can return the structure, but the slot-8 refinery wait is not applied unless the returned building type has `Refinery=yes`.

## Implementation Order

1. Add the private west-cell lookup helper and a small unit test if fixture setup is cheap.
2. Switch `phase_unloading` owner/event identity from `ref_sid` to rediscovered building.
3. Add the no-building null branch before cargo removal.
4. Run the state-3 focused tests before changing state 4.
5. Adjust `phase_departing` to keep bookkeeping and state-4 building identity separate.
6. Add the caller-side `Refinery=yes` guard only for state-4 slot-8 wait logic that already exists or is introduced by a separate verified patch.
7. Add/adjust the remaining tests.
8. Run focused miner tests first, then broader sim tests if compilation succeeds.

Suggested focused commands:

```powershell
cargo test -q sim::miner::miner_tests::unload_state3_uses_west_cell_building_not_reserved_refinery
cargo test -q sim::miner::miner_tests::missing_west_cell_building_does_not_credit_or_emit_deposit_event
cargo test -q sim::miner::miner_tests::state3_null_lookup_preserves_full_cargo_and_returns_to_refinery_selection
cargo test -q sim::miner::miner_tests::state3_null_lookup_does_not_clear_unload_display_latch
cargo test -q sim::miner::miner_tests::reserved_refinery_released_but_not_used_for_unload_credit_identity
cargo test -q sim::miner::miner_tests::state4_refinery_yes_guard_is_caller_owned
```

If test module paths differ, use the closest `cargo test -q <test_name>` filter.

## Non-Goals

- Do not implement or refactor radio `0x16`.
- Do not change far-return fallback search.
- Do not delete `reserved_refinery`.
- Do not introduce `ReleaseDockedHarvester` or `Force_Track(0x47)` into normal stock unload completion.
- Do not solve exact stale visual frame count without runtime evidence.
- Do not change per-bale/storage-slot credit mechanics in this patch unless required by the lookup refactor.

## Risks

- The existing occupancy list may not perfectly encode gamemd `CellClass+0xE4` order in every exotic multi-object case. It is still the best current deterministic object-list equivalent and is closer than direct `reserved_refinery`.
- The null-refinery branch may interact with existing invalid-refinery cleanup that clears display override too early. Keep that branch explicit instead of routing through a broad cleanup helper unless the helper is adjusted.
- Some tests may need fixture helpers to place structures across all occupied foundation cells so `sim.occupancy` mirrors the entity store.
- The helper returning any structure is intentional. Filtering to `Refinery=yes`
  in the helper would hide state-specific behavior and diverge from the generic
  `Look_up_building_in_cell` mechanism.

## Done Criteria

- State 3 credits/events use the building west of the miner.
- Missing west-cell building preserves cargo and emits no credit/deposit event.
- Missing west-cell building does not fall back to miner-owner credits.
- `reserved_refinery` remains only reservation/contact bookkeeping during unload state 3/4.
- State-4 refinery-specific wait logic checks `Refinery=yes` at the caller, not in the lookup helper.
- Focused tests cover mismatch, missing-building, and bookkeeping split scenarios.
