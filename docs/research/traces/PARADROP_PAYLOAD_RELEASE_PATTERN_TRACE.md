# Paradrop Payload Release Pattern Trace

**Scenario:** Loaded standard-YR PDPLANE drops 4 infantry while flying over target cell `(50,20)` on passable ground.
**Scope:** payload release only: V-pattern offsets, payload count ordering, cargo pop/retry behavior, passenger role/liveness transition, final cargo state.
**Assumption needed by prompt:** the house edge/facing was not specified. I used the current default `HouseState::waypoint_edge = 0` north-edge path, so the carrier is treated as flying south over the target: `facing=128`, aircraft lepton coord `(50*256+128,20*256+128)`.
**Binary activity check:** `AircraftClass::Drop_Payload @ 0x00415C60` is active in standard YR through `Mission_Rescue @ 0x00415960` for standard `Type=ParaDrop` / `Type=AmerParaDrop` PDPLANEs. This is not dormant TS legacy.

## Pipeline

`Mission_Rescue tick ready` -> `Drop_Payload/try_drop` -> `pop cargo head` -> `payload_count--` -> `V target cell from post-decrement parity` -> `cell/passability/subcell placement` -> `Unlimbo / begin descent` -> `sound + cargo/payload final state`

## Concrete Values

Initial scoped state:

- Aircraft position: cell `(50,20)`, subcell `(128,128)`, world leptons `(12928,5248)`.
- Aircraft facing: `128` south.
- Cargo FIFO/head order: `P1,P2,P3,P4`.
- Initial payload count: `4`.
- Ground: passable, empty, no building, no vehicle, no existing infantry occupancy.

V-target math shared by Rust and gamemd before infantry subcell placement:

| Drop | Popped | Pre count | Post count | Parity | Side relative to facing south | V target leptons | V target cell/sub |
|---|---|---:|---:|---|---|---|---|
| 1 | P1 | 4 | 3 | odd | left/east | `(13056,5248)` | `(51,20)` sub `(0,128)` |
| 2 | P2 | 3 | 2 | even | right/west | `(12800,5248)` | `(50,20)` sub `(0,128)` |
| 3 | P3 | 2 | 1 | odd | left/east | `(13056,5248)` | `(51,20)` sub `(0,128)` |
| 4 | P4 | 1 | 0 | even | right/west | `(12800,5248)` | `(50,20)` sub `(0,128)` |

gamemd does not use those V coordinates as the final infantry XY. It passes the V target cell and incoming coordinate through `CellClass::PlaceInfantryInCell @ 0x00481180`. For these exact `(sub_x=0, sub_y=128)` inputs, the subcell quadrant resolves to `0`, then `RandomRanged(0,3)` selects a rotation over valid functional subcells. On empty ground, possible final subcell offsets are:

- subcell 2: `(192,64)`
- subcell 3: `(64,192)`
- subcell 4: `(192,192)`

Rust currently uses the raw V coordinate directly: `(0,128)`. That is not one of the valid gamemd infantry subcell positions.

## Stage Verdicts

| Stage | Verdict | Rust output | gamemd output | Evidence |
|---|---|---|---|---|
| Ready drop entry | PASS | `advance_tick` calls `try_drop` when Rescue-equivalent mutation has `paradrop_try_drop` | `Mission_Rescue` calls `Drop_Payload` in range and returns `5` | Rust `src/sim/aircraft/mod.rs:717`; gamemd `0x004159FB -> 0x00415C60` |
| Cargo pop ordering | PASS | `PassengerCargo::unload_first` removes `passengers[0]`; sequence `P1,P2,P3,P4` | `FUN_00473430` pops cargo head and advances head pointer | Rust `src/sim/passenger.rs:99`; gamemd `0x00473430` |
| Payload count ordering | PASS | `payload_count_post = pre.saturating_sub(1)` before V parity; mission count decrements only on success | `aircraft+0x2FC` decremented before parity; restored on failure | Rust `src/sim/aircraft/drop_payload.rs:137`; `src/sim/aircraft/mod.rs:738`; gamemd `0x00415C60` |
| V-pattern pre-placement target | PASS | For facing 128 and count `4,3,2,1`, target alternates east/west by exactly 128 leptons | Same post-decrement parity and 128-lepton radius | Rust `src/sim/aircraft/drop_payload.rs:25`; gamemd `0x00415C60`, constant `0x007E2808 = 128.0` |
| Final infantry XY placement | FAIL | Infantry is assigned raw V coordinate `(sub_x=0, sub_y=128)` | Infantry is placed by `PlaceInfantryInCell`, yielding valid subcell 2/3/4, never `(0,128)` on empty ground | Rust `src/sim/aircraft/drop_payload.rs:166`; gamemd `0x00415C60 -> 0x00481180` |
| Impassable-cell retry | PASS | If `path_grid.is_walkable` is false, passenger is inserted back at cargo head and mission payload count remains unchanged | If `Can_Enter_Cell` fails, passenger is re-added to cargo and `PayloadCount++` restores parity | Rust `src/sim/aircraft/drop_payload.rs:146`; `src/sim/aircraft/mod.rs:745`; gamemd `0x00415C60` |
| Subcell/Unlimbo failure retry | FAIL | Rust has no `PlaceInfantryInCell` allocation, so a full/invalid infantry subcell condition is not tested; it can proceed where gamemd retries | gamemd retries if `PlaceInfantryInCell` returns sentinel or `Unlimbo` fails | Rust `src/sim/aircraft/drop_payload.rs:147`; gamemd `0x00415C60`, `0x00481180` |
| Attach failure role restoration | PASS | If `begin_parachute_descent` fails, Rust restores `PassengerRole::Inside` and cargo head | gamemd calls `CargoClass::AddPassenger`, relimbos passenger, and restores payload count | Rust `src/sim/aircraft/drop_payload.rs:178`; gamemd `0x00415C60`, `0x004733A0` |
| Success passenger liveness/role | UNCHECKED | Rust sets `PassengerRole::None` and `parachute_state=Some` | gamemd `Unlimbo` succeeds and attaches `PARACH`; exact transport-pointer clear was not computed in this trace | Rust `src/sim/aircraft/drop_payload.rs:174`; `src/sim/movement/parachute_descent.rs:45`; gamemd `0x005F5940` |
| Final cargo state after four clean drops | PASS | Cargo list empty, `total_size` reduced by four E1 sizes, mission payload count `0` | Cargo head null/count zero, payload count `0` after four successful drops | Rust `src/sim/aircraft/drop_payload.rs:203`; gamemd `0x00473430`, `0x00415C60` |
| Last-drop bookkeeping | UNCHECKED | Rust emits sound and updates cargo, but has no obvious equivalents for last-drop frame/drop-cell scratch fields | gamemd writes `LandingState=5`, `LastDropFrame=g_CurrentFrameCounter`, last drop cell, scratch zero | Rust `src/sim/aircraft/drop_payload.rs:197`; gamemd `0x00415C60` |

## Findings

1. **FAIL - final infantry XY placement:** Rust drops infantry at the raw half-cell V coordinate. gamemd uses that as a candidate cell/input and then calls `PlaceInfantryInCell`, so the visible infantry appears at valid infantry subcells. In this scenario Rust uses `(0,128)` inside the cell, while gamemd uses one of `(192,64)`, `(64,192)`, `(192,192)`.
2. **FAIL - subcell placement failure retry:** Rust only checks `PathGrid::is_walkable`; it does not model the gamemd `PlaceInfantryInCell` sentinel failure path. If the V target cell is walkable but infantry subcells are unavailable, gamemd re-adds the same passenger to cargo and restores payload count; Rust proceeds with the drop.

## Adjacent Findings

- Cadence/mission entry was only checked enough to identify the active caller. Full timing belongs to the cadence trace.
- Descent rendering after `begin_parachute_descent` belongs to the infantry descent/render trace.
- The exact live binary transport pointer clear on successful `Unlimbo` was not resolved here; only observable cargo membership/liveness was traced.

## Verdict Tally

PASS: 6 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

