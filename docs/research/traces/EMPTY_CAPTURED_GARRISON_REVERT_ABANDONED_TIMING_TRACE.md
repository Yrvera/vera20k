# Empty Captured Garrison Revert / Abandoned Timing Trace

Date: 2026-05-27

Scenario: a captured civilian `CanBeOccupied` garrison is currently owned by a player and has exactly one occupant. The player performs the normal unload/abandon action and the last occupant leaves. This trace is limited to owner revert target plus `StructureAbandoned`/equivalent timing and owner.

Status: COMPLETE for owner/revert/event timing. Exact native exit-cell placement and scatter are adjacent findings only.

## Pipeline

1. Player normal unload command selects an occupied `CanBeOccupied` building.
2. Native building mission slot 26 (`0x0044D880`) runs during the building update and calls `BuildingClass::SellBuilding` when occupant count is positive.
3. `SellBuilding` ejects/clears the garrison vector; for this one-occupant scenario, count changes `1 -> 0`.
4. The same `BuildingClass::Update @ 0x0043FB20` later reaches the `CanBeOccupied` guard and calls `CheckAutoSellOrCivilian @ 0x00458200`.
5. `CheckAutoSellOrCivilian` sees `count == 0 && owner != civilian_house`, emits abandoned cues for the pre-revert owner, refreshes anim state, then `ChangeOwner(civilian_house, 0)`.
6. Rust UI emits `Command::UnloadPassengers`; command dispatch sets `OrderIntent::Unloading`.
7. Rust `tick_passenger_system` first runs boarding/order reconciliation over sorted stable ids, then runs `tick_unloading`; no post-unload reconciliation runs in that tick.

## Active-YR Confirmation

- `BuildingClass::Update @ 0x0043FB20` is active standard YR building update and calls `CheckAutoSellOrCivilian` when `Type+0x157B` (`CanBeOccupied`) is nonzero.
- `CheckAutoSellOrCivilian @ 0x00458200` is active for scoped civilian garrisons after the `Type+0x634 == -1` gate. It calls `SellBuilding` for red HP, resolves the Civilian-side house, then performs occupied transfer or empty revert.
- `BuildingClass::Mission slot 26 @ 0x0044D880` has a mission-table data xref at `0x007E40F8`; `(0x007E40F8 - 0x007E4090) / 4 = 26`. Its first action is `GetOccupantCount() > 0 -> SellBuilding`.
- Standard YR has many active `CanBeOccupied=yes` buildings; for example `ini/rulesmd.ini:19302` `[CAGAS01]`, `ini/rulesmd.ini:19322` `CanBeOccupied=yes`, `ini/rulesmd.ini:19323` `MaxNumberOccupants=10`.

## Stage Verdicts

| Stage | gamemd result | Current Rust result | Verdict |
|---|---|---|---|
| Normal unload entry point | Building mission slot 26 calls `SellBuilding` from the building's own update when occupant count is positive. Evidence: Ghidra `0x0044D880`, xref `0x007E40F8`. | `app_context_order.rs:188..193` creates `Command::UnloadPassengers`; `world_commands.rs:903..917` sets `OrderIntent::Unloading`. | FAIL |
| Last occupant count mutation timing | Count `1 -> 0` occurs inside the building mission before the later `CheckAutoSellOrCivilian` call in the same `BuildingClass::Update`. Evidence: `0x0044D880`, `0x0043FB20`. | `passenger.rs:266..270` calls reconciliation before `tick_unloading`; `passenger.rs:650..816` unloads afterward and returns `false`. | FAIL |
| Empty revert trigger timing | Same building update invocation can emit abandoned cues and call `ChangeOwner` after normal unload empties the vector. | No same-tick post-unload reconciliation. Empty revert waits until a later call to `reconcile_civilian_garrison_owner_for_building`, normally next tick. | FAIL |
| Revert target | Target is the resolved Civilian-side `HouseClass*`: side lookup, house array scan, then `ChangeOwner(civilian_house, 0)`. It does not read a stored per-building original owner. Evidence: `0x00458200`. | `resolved_civilian_garrison_owner` returns `Neutral` if houses are empty or contain Neutral, otherwise `Special`, otherwise interns Neutral (`passenger.rs:422..433`). No side/country scan. For a stock roster with `Neutral` this likely lands on the same name, but literal native house-pointer equality was not runtime-computed. | UNCHECKED |
| Abandoned cue owner | Pre-revert building owner: `CheckAutoSellOrCivilian` performs human/sound/radar/EVA branch before `ChangeOwner`. For this scenario input, owner is the capturing player. | `StructureAbandoned { owner: current_owner }` is pushed before `building.owner = civilian_owner` (`passenger.rs:479..486`). For this scenario input, owner is the capturing player. | PASS |
| Abandoned cue timing | Native abandoned cue is emitted in the same building update after normal unload empties the vector, before `ChangeOwner`. | Rust event is emitted only when reconciliation later sees empty cargo; because normal unload runs after reconciliation, the event is delayed at least one `tick_passenger_system` call. | FAIL |
| Ownership changed propagation | Native owner changes during the same building update, so house color/control can update from that frame's post-update state. | `TickResult.ownership_changed` receives `passenger_ownership_changed` from `tick_passenger_system` (`world/mod.rs:1535..1538`, `1674..1680`); unload tick returns no owner change. | FAIL |
| Local EVA gating | Native checks `HouseClass::IsHumanPlayer` before abandoned EVA/audio/radar. | Rust sim emits a pure `StructureAbandoned` event; app layer filters by `local_owner_name` before resolving `EVA_StructureAbandoned` (`app_sim_tick.rs:475..492`). For a local human player, audible owner matches; full radar-event equivalence was not computed. | UNCHECKED |

Verdict tally: PASS: 1 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Failures

1. Stage 1/2 - Normal unload is scheduled through the wrong Rust phase. Native runs garrison ejection from building mission slot 26 inside the building's update; Rust defers player normal unload to global passenger phase after reconciliation. Player-visible effect: the emptied building remains player-owned for at least one extra Rust sim tick.
2. Stage 3/6 - `StructureAbandoned`/equivalent cue timing is late. Native emits the abandoned cue before `ChangeOwner` in the same building update that performs normal unload; Rust emits it only on the next reconciliation pass.
3. Stage 7 - `ownership_changed` is false on the Rust unload tick. Rendering/selection/control systems that key on `TickResult.ownership_changed` cannot see the native same-update owner revert.
4. Stage 4 - Revert target mechanism is not native. Native resolves the Civilian-side house through side/house data; Rust uses a `Neutral`/`Special` name preference. Concrete stock output was not runtime-proven equal, so this remains UNCHECKED for the exact stock house pointer but is a mechanism drift risk.

## Current Rust Touchpoints

- `src/app_context_order.rs:188..193` emits `Command::UnloadPassengers` for a selected occupied `CanBeOccupied` structure.
- `src/sim/world/world_commands.rs:903..917` validates ownership/passenger presence and sets `OrderIntent::Unloading`.
- `src/sim/passenger.rs:266..270` runs reconciliation before unloading.
- `src/sim/passenger.rs:279..300` reconciles buildings only during the pre-unload ordered walk.
- `src/sim/passenger.rs:650..816` pops one passenger, restores it to the map, clears unload order when empty, and returns `false`.
- `src/sim/passenger.rs:479..486` emits `StructureAbandoned` with pre-revert owner then writes civilian owner, but only when reconciliation is called after cargo is already empty.
- `src/sim/world/mod.rs:1535..1538` runs passenger processing in phase 6 after retaliation.

## Adjacent Findings

- Native `SellBuilding` exit scan, coordinate reuse, LIFO occupant order, and post-unlimbo scatter are verified elsewhere and differ from several Rust ejection approximations. This trace did not re-trace those mechanics.
- Native red-HP and destruction callers also use `SellBuilding`, but they are not this normal-unload scenario.
- Player sell state machine behavior is adjacent and not part of this trace.

## Tests / Verification

No Cargo tests were run. Active `cargo`/`rustc` processes were present (`cargo` PIDs 22160 and 32456; `rustc` PID 29760), so starting another test run would violate the project build-coordination rule.

## Sources

- Read-only Ghidra decompile: `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200`, `BuildingClass::Update @ 0x0043FB20`, `BuildingClass::SellBuilding @ 0x00457DE0`, `BuildingClass` mission slot 26 body `0x0044D880`, `LogicClass::PerTickUpdate @ 0x0055AFB0`.
- Read-only Ghidra xref: `0x00457DE0` callers include `0x0044D89C`, `0x00458229`, `0x0044A5CA`, `0x0044263B`; `0x0044D880` has data xref `0x007E40F8`.
- Research docs: `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `docs/research/CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`, `docs/research/GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`, `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`.
- Rust source: `src/sim/passenger.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_commands.rs`, `src/app_context_order.rs`, `src/app_sim_tick.rs`.
