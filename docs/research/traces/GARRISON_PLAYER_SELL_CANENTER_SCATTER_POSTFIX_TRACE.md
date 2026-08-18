# Garrison Player-Sell CanEnter Scatter Postfix Trace

**Scenario:** A player sells a captured civilian `CanBeOccupied` garrison with multiple infantry occupants.
**Concrete Rust fixture for numeric stages:** `CAGAS01` at `(rx,ry)=(10,10)`, `Foundation=2x2`, three hidden occupants in cargo order `[pax1,pax2,pax3]`, normal player-sell mode, no exterior blockers unless the stage explicitly says all perimeter cells are rejected.
**Scope:** LIFO occupant order, one chosen exit coordinate, first occupant `Can_Enter_Cell(cell,-1,-1,0,1)==0` probe, no-exit player-sell fallback, and direct Scatter RNG gating after placement.
**Non-scope:** destruction fallback, normal abandon/unload, crew survivors, full Infantry scatter destination search, render/audio after sell.

## Evidence Used

- Ghidra read-only decompile in this trace:
  - `BuildingClass__SellBuilding @ 0x00457DE0`
  - `BuildingClass__Sell @ 0x00449C30`
  - `InfantryClass__Scatter @ 0x0051D0D0`
- Verified prior reports:
  - `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`
  - `docs/research/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
  - `docs/research/CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md`
  - `docs/research/INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`
- Rust source read-only scan:
  - `src/sim/production/production_sell.rs`
  - `src/sim/pathfinding/cell_entry.rs`
  - `src/sim/occupancy.rs`

## Active YR Confirmation

This path is active in standard Yuri's Revenge:

- `rulesmd.ini` contains active civilian `CanBeOccupied=yes` buildings, including `[CAGAS01]` with `MaxNumberOccupants=10`.
- `artmd.ini` has `[CAGAS01]` with `Foundation=2x2`.
- `BuildingClass__Sell @ 0x00449C30` state 1 reaches `BuildingClass__SellBuilding @ 0x00457DE0` when the garrison occupant count is positive.
- The `SellBuilding` helper reads the garrison occupant vector, probes the first occupant through vtable `+0x1AC`, unlimbos occupants, and calls the occupant Scatter virtual. No TS-only gate was found on this player-sell path.

## Pipeline

`player sell command` -> `BuildingClass::Sell state 1 / Rust sell_building` -> `SellBuilding-style ejection helper` -> `edge scan using occupant slot 0 Can_Enter_Cell` -> `one selected coordinate or player-sell fallback` -> `reverse occupant placement` -> `direct Scatter handoff/RNG subset` -> `building sell removal/refund continues`

## Stage Results

| Stage | gamemd output for scoped scenario | Current Rust output | Verdict |
|---|---|---|---|
| 1. Player-sell garrison entry | If occupant count is positive, `BuildingClass__Sell` calls `SellBuilding` once during state 1. | `sell_building` calls `eject_garrison_occupants` once before removing/refunding the building (`production_sell.rs:576..584`). | PASS |
| 2. 2x2 edge scan order | For `(10,10)` and `2x2`, the candidate sequence is `(12,12),(12,11),(12,10),(12,9),(12,12),(11,12),(10,12),(9,12),(10,9),(11,9),(12,9),(9,10),(9,11),(9,12)`. | `garrison_sellbuilding_exit_cells` emits the same 14 entries; the focused test fixes the exact vector (`production_sell.rs:243..269`, `823..844`). | PASS |
| 3. First occupant only | Native loads `*(this+0x688)[0]` before the scan and repeatedly calls that occupant's vtable `+0x1AC`; later occupants do not drive candidate acceptance. | `choose_garrison_exit_cell` reads `passenger_ids.first()` once and probes only that id (`production_sell.rs:304..315`, `854..870`). | PASS |
| 4. Accepted-cell predicate mechanism | Native accepts only when first occupant `Can_Enter_Cell(CellClass*,-1,-1,0,1)` returns `0`. This is full InfantryClass vtable `+0x1AC`, not just occupancy. | Rust uses `check_terrain((rx,ry), Ground, first_occupant.category, None, None, &sim.occupancy) == Clear` (`production_sell.rs:272..301`). That skips map terrain/cost grids, bridge/layer context, owner/blocker classification, infantry building policy, low-bridge tube logic, and other verified `0x0051BF90` branches. | FAIL |
| 5. Selected coordinate in empty-exterior fixture | If native accepts the first candidate, the chosen coordinate is `(12,12)` and one coordinate is reused. I did not capture a live gamemd return for `Can_Enter_Cell((12,12),-1,-1,0,1)` in this exact fixture. | With no exterior blockers and an inside live first occupant, Rust chooses `(12,12)`. | UNCHECKED |
| 6. Single coordinate reuse | Native converts the selected cell once before the occupant loop, then reuses the same unlimbo coordinate for every occupant. | `exit_cell` is chosen once, then `exit_rx, exit_ry` is passed to every placement (`production_sell.rs:425..455`). | PASS |
| 7. Occupant order | Native loop decrements from count-1 to 0, so cargo `[pax1,pax2,pax3]` ejects as `pax3,pax2,pax1`. | Rust iterates `passenger_ids.iter().rev()` (`production_sell.rs:442..458`). | PASS |
| 8. Player-sell no-exit fallback | Normal player sell uses the inside-foundation fallback coordinate; for `(10,10)` `2x2`, this is `(11,11)`. Destruction null removal is a separate caller mode. | `GarrisonEjectMode::PlayerSell` selects `garrison_inside_foundation_fallback`, which returns `(rx+width-1, ry+height-1)` = `(11,11)` (`production_sell.rs:318..322`, `425..430`, `1057..1086`). | PASS |
| 9. Direct Scatter call placement order | After successful unlimbo, native clears archive-like state, calls vtable `+0x3C8(0)`, computes the building coordinate, then calls occupant vtable `+0x174(building coord,1,1)`. | Rust places the entity, adds occupancy, then calls `sellbuilding_direct_scatter_handoff` (`production_sell.rs:361..405`). It does not model `+0x3C8(0)` or the `+0x174` call target/state write. | NOT-IMPLEMENTED |
| 10. Scatter RNG range subset | In active `InfantryClass__Scatter`, after its gates, the garrison handoff reaches `Random__RandomRanged(0,4)` before passable-cell search and mission/destination writes. | Rust consumes exactly `next_range_u32_inclusive(0,4)` after its local gates (`production_sell.rs:333..359`, `1088..1126`). | PASS for range and post-placement position; FAIL for gate equivalence, covered by stage 11 |
| 11. Scatter gate equivalence | Native gates include player-control/mission cases, locomotion pointer, mission timer entry, nav target/ShouldNotScatter, global Scatter setting, weapon/player checks, and passability search before mission writes. | Rust gates only category infantry, alive, not dying, not still inside, and has locomotor (`production_sell.rs:345..352`). This can consume or skip the RNG at different times than gamemd. | FAIL |
| 12. Scatter downstream side effects | Native can find a nearby passable cell, queue mission `2`, and set a destination/focus after the RNG draw. | Rust intentionally leaves destination, mission, and archive-target storage unset (`production_sell.rs:354..358`, `1117..1125`). | NOT-IMPLEMENTED |

## Verdict Tally

PASS: 7 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 2

## Failures And Missing Work

1. **Stage 4 - accepted-cell predicate FAIL.** Player-visible difference: Rust can accept or reject a perimeter cell differently from native `InfantryClass::Can_Enter_Cell`, changing where all occupants appear or whether the player-sell fallback fires. Rust: `src/sim/production/production_sell.rs:272`; gamemd: `BuildingClass__SellBuilding @ 0x00457DE0` calls first occupant vtable `+0x1AC` with `CellClass*,-1,-1,0,1`.
2. **Stage 11 - Scatter gate equivalence FAIL.** Player-visible difference: Rust can consume the scatter RNG when native would return early, or skip it when native would scatter, shifting later RNG and infantry movement. Rust: `src/sim/production/production_sell.rs:345`; gamemd: `InfantryClass__Scatter @ 0x0051D0D0` gate chain before `Random__RandomRanged(0,4)`.
3. **Stage 9 - direct Scatter call NOT-IMPLEMENTED.** Player-visible difference: Rust does not model the native `+0x3C8(0)` handoff or true `+0x174(building coord,1,1)` Scatter call state. Rust: `src/sim/production/production_sell.rs:333`; gamemd: `BuildingClass__SellBuilding @ 0x004580E9..0x0045810A`.
4. **Stage 12 - Scatter downstream side effects NOT-IMPLEMENTED.** Player-visible difference: ejected infantry do not receive native passable-cell destination/mission side effects after the RNG draw. Rust: `src/sim/production/production_sell.rs:354`; gamemd: `InfantryClass__Scatter @ 0x0051D487..0x0051D694`.

## Adjacent Findings

- Full Infantry `Can_Enter_Cell @ 0x0051BF90` remains only partially represented by the shared Rust phase-1 check; low bridges, building/garrison classifier returns, and exact terminal subcell-full return ladder are adjacent to this trace.
- Map-edge negative candidate handling is adjacent. Rust drops negative candidate cells through `push_nonnegative_cell`; this run did not trace native `MapClass__Get_CellClass` behavior for map-edge garrisons.
- Destruction no-exit null removal is adjacent and was intentionally excluded from this player-sell trace.
- Full Scatter destination search, mission queue, and archive-target state need a separate implementation contract before code writes; this trace only checks the player-sell handoff and RNG subset.

## Verification Notes

- I did not run Cargo or any executable test because this subagent was constrained to write exactly one file.
- The selected-coordinate stage remains `UNCHECKED`: Rust selects `(12,12)` in the empty exterior fixture, and native would do the same if the first `Can_Enter_Cell` returns `0`, but this run did not capture the exact live gamemd return for that cell.

## Status

COMPLETE
