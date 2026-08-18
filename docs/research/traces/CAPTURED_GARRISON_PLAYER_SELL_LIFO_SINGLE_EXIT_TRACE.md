# Captured Garrison Player-Sell LIFO Single Exit Trace

**Scenario:** captured 2x2 civilian `CanBeOccupied` garrison with three infantry occupants, currently owned by the selling player, sold through the player sell command.
**Concrete Rust fixture used for numerical comparison:** `CAGAS01` at `(rx,ry)=(10,10)`, `Foundation=2x2`, occupant vector `[pax1,pax2,pax3]`, no exterior blockers in the Rust fixture.
**Scope:** player-sell `SellBuilding` garrison occupant ejection only: edge scan order, selected single exit coordinate, LIFO order, owner handling, mission/scatter gate, and RNG consumption.
**Non-scope:** destruction no-exit fallback, generic transport unload, survivor crew spawning, full Infantry scatter branch table, rendering/audio after final building sale.
**Status:** COMPLETE for this trace report; one selected-coordinate substage remains `UNCHECKED` because no live gamemd `Can_Enter_Cell` return was captured for the exact map cell.

## Evidence Used

- Ghidra read-only decompile: `BuildingClass__SellBuilding`, `BuildingClass__Sell`, `InfantryClass__Scatter`, `Random__RandomRanged`.
- Ghidra read-only caller context:
  - `0x0044A5CA`: player sell calls `SellBuilding`; context shows `PUSH 0x1`, `PUSH 0x0`, then `CALL 0x00457DE0`. With normal stack order this means first argument `0`, second argument `1`.
  - `0x0044263B` and `0x00458229`: destruction/red-HP callers push `0,0`; used only to confirm player-sell argument split is not a TS/dormant path.
- Verified research:
  - `docs/research/GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`
  - `docs/research/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
  - `docs/research/CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md`
- Rust source: `src/sim/production/production_sell.rs`.

## Active YR Confirmation

This is active in standard Yuri's Revenge, not dormant TS code:

- Standard YR data contains active `CanBeOccupied=yes` civilian buildings; local test fixture uses `CAGAS01` with `Foundation=2x2`, `CanBeOccupied=yes`, `MaxNumberOccupants=5` (`production_sell.rs:660..668`).
- Player sell reaches `BuildingClass__Sell @ 0x00449C30`; state 1 calls `BuildingClass__SellBuilding @ 0x00457DE0` when occupant count is positive.
- Captured-civilian sell outcome is active after ownership reconciliation: sell-mode uses current human owner and `BuildingClass__Sell` has no captured-civilian preserve/revert branch.

## Pipeline

`player sell command` -> `BuildingClass::Sell state 1 / Rust sell_building` -> `SellBuilding-style garrison helper` -> `choose one exit cell` -> `LIFO unlimbo/place all occupants at same coordinate` -> `native direct Scatter handoff / Rust currently no-op` -> `native sell continues to remove/refund building / Rust removes/refunds building`

## Stage Results

| Stage | gamemd output for scoped scenario | Current Rust output | Verdict |
|---|---|---|---|
| 1. Player-sell ejection entry | Occupant count `3 > 0`; player sell calls `SellBuilding` once with first arg `0`, second arg `1` (`0x0044A5BA..0x0044A5CA`). | `sell_building` calls `eject_garrison_occupants` once before removing the building (`production_sell.rs:523..531`). | PASS |
| 2. 2x2 edge scan order | For origin `(10,10)`, `W=2`, `H=2`: `(12,12),(12,11),(12,10),(12,9),(12,12),(11,12),(10,12),(9,12),(10,9),(11,9),(12,9),(9,10),(9,11),(9,12)`. | `garrison_sellbuilding_exit_cells(10,10,2,2)` test expects exactly that vector (`production_sell.rs:739..758`). | PASS |
| 3. Accepted-cell predicate | First occupant vtable `+0x1AC(MapCell,-1,-1,0,1) == 0`; this is the native `Can_Enter_Cell`-style predicate. | `garrison_first_occupant_can_enter_cell_approx` only checks first occupant liveness/inside state plus no live entity already at the cell (`production_sell.rs:270..297`). | FAIL |
| 4. Selected single exit coordinate | If native first probe accepts, chosen cell is `(12,12)` and one coordinate is reused for all occupants. | With no exterior blockers, Rust chooses `(12,12)` (`production_sell.rs:299..313`, fixture expectation at `870..910`). Exact gamemd predicate return for this concrete map cell was not captured live. | UNCHECKED |
| 5. Single coordinate reuse | One chosen coordinate is converted before the loop and reused for every occupant (`0x00458060..0x004580BD`). | `exit_cell` is chosen once, then every passenger is placed at `exit_rx,exit_ry` (`production_sell.rs:383..403`). | PASS |
| 6. Occupant order | Reverse vector order: `count-1` down to `0`, so `[pax1,pax2,pax3]` ejects as `pax3,pax2,pax1` (`0x00458098..0x0045819E`). | `for &pax_id in passenger_ids.iter().rev()` (`production_sell.rs:400..403`). | PASS |
| 7. Owner handling | `SellBuilding` does not `ChangeOwner`; player-sell path uses current player ownership and then final sell removes/refunds the building. | Player sell passes `owner_override=None`, so occupants keep their owner; helper does not revert `garrison_original_owner`; `sell_building` removes/refunds (`production_sell.rs:402,444..453,523..543`). | PASS |
| 8. Mission/scatter handoff | After successful `Unlimbo`, native calls `+0x3C8(0)`, then occupant Scatter `+0x174(building coord,1,1)`. Player-sell first arg is `0`, so the later `+0x1E8(0xF,0)` block is not active for this direct caller. | `place_garrison_passenger_at_cell` directly writes position/occupancy and has a TODO for exact `+0x3C8`/`+0x174`; no Scatter handoff exists (`production_sell.rs:330..365`). | NOT-IMPLEMENTED |
| 9. RNG before Scatter | `SellBuilding` exit selection and unlimbo loop consume zero RNG draws. Any infantry RNG belongs inside the later Scatter call and uses scenario `RandomRanged(0,4)` after Scatter gates. | Current helper consumes zero RNG; local tests snapshot `sim.rng.state()` around ejection (`production_sell.rs:881..900`, `955..966`). | PASS for pre-Scatter zero-draw contract; Scatter RNG is covered by Stage 8 NOT-IMPLEMENTED |

## Verdict Tally

PASS: 6 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Failures And Missing Work

1. **Stage 3 - accepted-cell predicate FAIL.** Player-visible difference: Rust can select `(12,12)` or another edge cell that gamemd's first occupant `Can_Enter_Cell(cell,-1,-1,0,1)` would reject, changing where all three occupants appear or whether player-sell falls back inside the foundation. Rust: `src/sim/production/production_sell.rs:270`; gamemd: first occupant vtable `+0x1AC` in `SellBuilding @ 0x00457E77..0x00457E99` and repeated loop sites.
2. **Stage 8 - mission/scatter handoff NOT-IMPLEMENTED.** Player-visible difference: ejected infantry appear at the exit coordinate but do not run native direct Scatter behavior, so movement, mission queue state, and later scatter RNG timing can differ. Rust: `src/sim/production/production_sell.rs:363`; gamemd: `SellBuilding @ 0x004580E9..0x0045810A`; infantry RNG path in `InfantryClass::Scatter @ 0x0051D2AC..0x0051D2BA`.

## Adjacent Findings

- Map-edge negative candidate behavior is adjacent, not traced here. Rust drops negative cells in `push_nonnegative_cell`; gamemd has no explicit SellBuilding clamp and delegates to `MapClass__Get_CellClass`.
- Destruction/red-HP no-exit behavior is adjacent. This trace only covers normal player sell where the second argument is `1` and no-exit fallback is inside-foundation `(ox+W-1,oy+H-1)`.
- Generic transport unload RNG is adjacent and remains outside this scenario.
- Full Infantry scatter table contents and all early returns are adjacent. This trace only needs the verified direct Scatter handoff and `RandomRanged(0,4)` classification.

## Verification Notes

- I did not run `cargo test` because the user constrained this subagent to write exactly one file, and a cargo run can update build artifacts outside the allowed report path.
- The selected-coordinate stage is intentionally `UNCHECKED`, not `PASS`: Rust's concrete fixture selects `(12,12)`, and gamemd would also select `(12,12)` if the first native `Can_Enter_Cell` call returns `0`, but I did not capture that exact native return value for this exact map cell in a live run.

## Status

COMPLETE
