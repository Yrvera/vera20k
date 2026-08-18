# Garrison Owner Order Before/After Entry Trace

Date: 2026-05-27

Scenario: one infantry unit enters one neutral `CanBeOccupied` civilian building. Compare only two relative update orders inside one global object pass:

- A: infantry entry commit occurs before the target building update.
- B: target building update occurs before infantry entry commit.

Do not generalize this trace to sell, destruction, red-health ejection, garrison fire, pips, visual anim variants, or arbitrary runtime vector indices.

## Verdict Summary

PASS: 2 | FAIL: 1 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

Overall status: COMPLETE.

Current Rust matches the two scoped order outcomes when the passenger/garrison reconciliation helper is supplied the same relative order as gamemd. Current production Rust still does not use gamemd's live `LogicClass` object-vector order as the source of that relative order; it uses sorted stable IDs. That can choose the wrong same-frame versus next-frame result for a concrete runtime pair if stable-ID order differs from gamemd object-vector order.

Tests were not run because the hard constraint allowed exactly one file write and Cargo would write under `target/`. Rust outputs below are computed from current source control flow and existing checked-in test assertions.

## Evidence Sources

- Native active-YR scheduler evidence: `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`.
- Native transfer timing evidence: `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`.
- Read-only Ghidra spot-checks in this run:
  - `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, caller `Main_Tick @ 0x0055D360`.
  - `BuildingClass__Update @ 0x0043FB20`.
  - `BuildingClass__CheckAutoSellOrCivilian @ 0x00458200`, caller `BuildingClass__Update`.
  - `BuildingClass__AddGarrisonOccupant @ 0x00522910`.
  - `InfantryClass__PerCellProcess @ 0x00519630`.
- Rust surfaces:
  - `src/sim/passenger.rs:266-298`, `303-415`, `436-490`, `1029-1081`.
  - `src/sim/world/mod.rs:1535-1539`, `1674-1680`.
  - `src/sim/entity_store.rs:1-4`, `23-33`, `107-108`.

## Active YR Confirmation

The native functions used here are active in standard Yuri's Revenge:

- `Main_Tick` calls `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- The object scheduler walks the main object vector forward, calling each object's `vtable+0x5C` update and reloading the live count after each call.
- `BuildingClass__Update @ 0x0043FB20` is the building `vtable+0x5C` body and calls `BuildingClass__CheckAutoSellOrCivilian @ 0x00458200` when `CanBeOccupied` is set.
- `InfantryClass__PerCellProcess @ 0x00519630` reaches `BuildingClass__AddGarrisonOccupant @ 0x00522910` for the normal infantry garrison entry path.
- `AddGarrisonOccupant` appends/limbos the occupant but does not transfer building owner.
- `CheckAutoSellOrCivilian` performs the owner transfer when occupant count is greater than zero and the building owner is the resolved civilian house.

These are not TS-only dormant paths for the scoped scenario; stock YR `rulesmd.ini` has active `CanBeOccupied=yes` civilian buildings, including `CAGAS01` at `ini/rulesmd.ini:19302` with `CanBeOccupied=yes` at `19322` and `MaxNumberOccupants=10` at `19323`, and stock `E1` has `Occupier=yes` at `ini/rulesmd.ini:3720`.

## Pipeline

Trigger: infantry reaches the target building cell during its object update.

Native chain: `LogicClass` live object vector turn for infantry -> `InfantryClass::PerCellProcess` -> `CanDock` -> `AddGarrisonOccupant` appends occupant -> later target building `vtable+0x5C` turn -> `BuildingClass::Update` -> `CheckAutoSellOrCivilian` -> `ChangeOwner(first_occupant.owner, 0)`.

Rust chain: `Simulation::advance_tick` phase 6 -> `passenger::tick_passenger_system` -> sorted stable-ID order -> for each ID, process boarding if the ID is a boarding passenger, then reconcile if the same ID is a building -> owner write in `reconcile_civilian_garrison_owner_for_building`.

## Stage Results

### Stage 1 - Native A: infantry entry before building update

Native input at frame T: building owner is the civilian/neutral house, occupant count is 0 before the infantry turn, first occupant owner is the infantry owner after entry.

Native computation: infantry turn calls `AddGarrisonOccupant`, changing occupant count from 0 to 1 and leaving owner unchanged. Later in the same live vector pass, the building turn calls `CheckAutoSellOrCivilian`; it reads count 1, current owner equal to civilian house, first occupant owner equal to the infantry owner, and calls `ChangeOwner(first_occupant.owner, 0)`.

Native output: owner transfer occurs in frame T, after entry but before the global pass ends. Frame delay after entry commit: 0.

Rust output for explicit order `[pax, bldg]`: `process_boarding_passenger` boards the passenger at `src/sim/passenger.rs:358-362`; owner is not written during boarding. The later building ID in the same helper call reaches `reconcile_civilian_garrison_owner_for_building`, reads non-empty cargo and civilian current owner at `src/sim/passenger.rs:463`, then writes `building.owner = new_owner` at `src/sim/passenger.rs:473-475`. Existing test `garrison_owner_transfers_same_frame_when_building_update_after_entry` asserts changed `true` and owner `"Americans"` at `src/sim/passenger.rs:1051-1061`.

Verdict: PASS for the scoped relative order. Native frame delay 0 equals Rust helper-pass delay 0 for the same order.

### Stage 2 - Native B: building update before infantry entry

Native input at frame T: building update runs first with owner still civilian/neutral and occupant count 0.

Native computation: the building's `CheckAutoSellOrCivilian` sees no occupant and does not transfer owner. Later in frame T, infantry entry calls `AddGarrisonOccupant`, increasing occupant count to 1 but still not changing owner. The target building has already had its update turn, so it does not reconcile again until its next building update, normally frame T+1.

Native output: owner remains civilian/neutral through frame T and transfers on the next target-building reconciliation pass, normally frame T+1. Frame delay after entry commit: 1.

Rust output for explicit order `[bldg, pax]`: first helper call reconciles the building before boarding and leaves owner `"Neutral"` with changed `false`; passenger then boards. The second helper call reaches the building first with non-empty cargo and changes owner to `"Americans"`. Existing test `garrison_owner_waits_next_frame_when_building_update_before_entry` asserts exactly that sequence at `src/sim/passenger.rs:1065-1081`.

Verdict: PASS for the scoped relative order. Native delay 1 equals Rust helper-pass delay 1 for the same order.

### Stage 3 - Production Rust source of relative order

Native input: the relative order is the current live `LogicClass` object-vector order. The scheduler starts at index 0, loads the object pointer from the item array, calls `vtable+0x5C`, increments the index, and reloads the live count. The research report cites `0x0055B5FB..0x0055B619`; the read-only Ghidra decompile in this run confirms the live forward loop in `LogicClassPerTickUpdateLiveVector`.

Rust input: production `tick_passenger_system` obtains `let order = sim.entities.keys_sorted();` at `src/sim/passenger.rs:266-268`. `EntityStore` is a `BTreeMap<u64, GameEntity>` keyed by stable ID and `keys_sorted()` returns ascending stable IDs at `src/sim/entity_store.rs:1-4`, `23-33`, `107-108`.

Rust computation: garrison owner timing is modeled by a local in-order surrogate, but production order is not gamemd object-vector order. The comment at `src/sim/passenger.rs:273-278` explicitly says full insertion-order plumbing is still a scheduler follow-up and production currently supplies stable-ID order.

Native output: same-frame versus next-frame transfer is selected by live object-vector order.

Rust output: same-frame versus next-frame transfer is selected by stable-ID order in production.

Verdict: FAIL. The scoped A/B branch behavior is present, but the production order source can disagree with gamemd for the same concrete infantry/building pair. Player-visible effect: building ownership, remap color, control, and owner-gated follow-on behavior can occur one frame too early or one frame too late when stable-ID order and gamemd object-vector order differ.

## Current Rust Match Answer

For the two explicitly supplied relative orders:

- A, infantry entry before building update: gamemd transfers owner in the same global frame; Rust's explicit-order helper also transfers in the same helper pass. PASS.
- B, building update before infantry entry: gamemd leaves owner neutral through that frame and transfers on the next building update, normally the next frame; Rust's explicit-order helper also waits until the next helper pass. PASS.

For production execution:

- Rust does not yet prove it will choose the same A or B order as gamemd for a concrete runtime pair because it derives the order from sorted stable IDs, not from the active `LogicClass` object vector. FAIL for global order-source parity.

## Failures

1. Stage 3 - Production order source drift: player can see ownership/recolor/control one frame early or late when Rust stable-ID order disagrees with gamemd object-vector order. Rust: `src/sim/passenger.rs:266-268`, `src/sim/entity_store.rs:107-108`. Gamemd evidence: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, forward object-vector loop cited at `0x0055B5FB..0x0055B619`.

## Not Implemented

None for the two explicit relative order outcomes. Full native scheduler/insertion-order plumbing remains a broader scheduler parity task, but this trace counts the current production order-source mismatch as FAIL rather than NOT-IMPLEMENTED because a working stable-ID order source exists and can produce a visible wrong frame.

## Adjacent Findings

- Empty garrison revert owner, red-health ejection, sell/destruction ejection, garrison fire, pips, and anim refresh were intentionally not traced here.
- Existing research says empty revert resolves the Civilian-side house rather than using per-building original-owner state; this trace did not re-audit that adjacent behavior.

## Status

COMPLETE for the requested concrete scenario. Remaining uncertainty about concrete retail map/replay object-vector indices does not change the mechanism: gamemd transfers on the first target-building reconciliation pass after the occupant vector changes.
