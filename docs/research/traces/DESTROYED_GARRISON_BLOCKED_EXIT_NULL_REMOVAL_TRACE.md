# Destroyed Garrison Blocked-Exit Null Removal Trace

**Scenario:** an occupied 2x2 civilian `CanBeOccupied` building is destroyed while all `SellBuilding` edge candidates are blocked.

**Concrete fixture used for numbers:** `CAEUR1` at origin cell `(10,10)`, one or more infantry occupants, all native `SellBuilding` perimeter candidates blocked. `CAEUR1` is a stock civilian garrison: `rules.ini:11424..11444` has `CanBeOccupied=yes`, `MaxNumberOccupants=3`; `art.ini:4270..4272` has `Foundation=2x2`.

**Scope:** no-exit destruction fallback only. Normal player sell inside-foundation fallback, successful edge ejection scatter internals, generic transport unload, and non-null parachute callers are adjacent findings only.

## Evidence

- Existing verified reports:
  - `docs/research/GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md`
  - `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`
- Fresh read-only Ghidra spot-check:
  - `BuildingClass__ReceiveDamage @ 0x00442230`
  - `BuildingClass__SellBuilding @ 0x00457DE0`
  - `SpawnUnitsWithParachute @ 0x004585C0`
- Rust surfaces:
  - `src/sim/combat/mod.rs:860..899`
  - `src/sim/world/mod.rs:1435..1438`
  - `src/sim/production/production_sell.rs:241..407`
  - `src/sim/production/production_sell.rs:480..495`
  - Tests at `src/sim/production/production_sell.rs:978..1008` and `src/sim/passenger.rs:1599..1633`

No Cargo test was run in this subagent pass because active `cargo`/`rustc` processes were present at startup and no code was changed.

## Pipeline

`ReceiveDamage destruction case` -> `CanBeOccupied gate` -> `SellBuilding edge scan` -> `no accepted edge cell` -> `SpawnUnitsWithParachute(0)` null branch -> reverse occupant destroy/remove -> vector/list cleanup -> no scatter, no mission, no RNG, no parachute.

Current Rust path:

`combat death loop` -> `DestroyedGarrisonBuilding` event -> `world drains event` -> `eject_destruction_garrison` -> same edge scan helper -> `DestructionNoExitRemove` -> reverse occupant `health=0`, `dying=true`, `PassengerRole::None` -> no scatter, no mission, no RNG, no parachute.

## Stage Trace

| Stage | Native output | Rust output | Verdict |
|---|---|---|---|
| Concrete data | `CAEUR1`: `CanBeOccupied=yes`, max occupants `3`, art foundation `2x2` | Rust fixture uses `CAGAS01` test data with `Foundation=2x2`, `CanBeOccupied=yes`; production rules merge art foundations through `RuleSet::merge_art_data` | PASS for dimensions/occupiable flag in the scoped 2x2 case |
| Destruction entry | `ReceiveDamage` case 4 checks `Type+0x157B`; if true, calls `SellBuilding` | death loop checks `obj.can_be_occupied && Structure && passenger_ids != empty`, emits `DestroyedGarrisonBuilding` | PASS for boolean gate in this scenario |
| Edge candidate order | For origin `(10,10)`, `W=2`, `H=2`: `(12,12),(12,11),(12,10),(12,9),(12,12),(11,12),(10,12),(9,12),(10,9),(11,9),(12,9),(9,10),(9,11),(9,12)` | `garrison_sellbuilding_exit_cells(10,10,2,2)` yields the same 14-cell sequence in its unit test | PASS |
| Candidate predicate | Native calls first occupant vtable `+0x1AC(CellClass*, -1, -1, 0, 1)`; zero accepts. In this scenario all 14 candidates are blocked, so no candidate is accepted. | Rust uses `garrison_first_occupant_can_enter_cell_approx`, rejecting live occupied cells; with blockers on all 14 candidates, no candidate is accepted. | UNCHECKED for exact predicate mechanism; PASS for the concrete all-blocked accepted-cell count `0 == 0` |
| No-exit fallback selector | Destruction callers pass the zero second argument, so no accepted candidate executes `SpawnUnitsWithParachute(0)` and returns. Active in standard YR. | `eject_destruction_garrison` passes `GarrisonEjectMode::DestructionNoExitRemove`, so no accepted candidate selects `None`. | PASS for selected branch result |
| Null fallback behavior | `SpawnUnitsWithParachute(0)` resets fire index, reverse-iterates occupants from `count-1` to `0`, calls occupant vtable `+0xF8`, then clears the vector/list. | Rust reverse-iterates `passenger_ids.iter().rev()` and marks each passenger removed with `health.current=0`, `dying=true`, `PassengerRole::None`; returns `0` ejected. | PASS for order and ejected count; UNCHECKED for exact occupant destroy/remove bytes after native `+0xF8` |
| Scatter and mission | Null branch has no `Unlimbo`, no `+0x3C8`, no `+0x174`, no `+0x1E8`, no landing/parachute mission. | The no-exit `None` branch returns before `place_garrison_passenger_at_cell`; no scatter/mission call exists on this branch. | PASS |
| RNG consumption | Null branch contains no `RandomRanged`, `Random::Next`, timer-derived direction choice, or modulo selection. Expected consumption: `0` draws. | `mark_garrison_passenger_removed` does not access `sim.rng`; focused test asserts `sim.rng.state()` unchanged. Expected consumption: `0` draws. | PASS |
| Parachute/chute visuals | No `ObjectClass::Unlimbo`, no falling state, no `PARACH` anim, no `ChuteSound`; helper name is misleading for the null argument. | Destruction no-exit path does not call `begin_parachute_descent` or set `parachute_state`; app chute polling only follows entities with `parachute_state`. | PASS |
| Timing/order relative to death | Native calls `SellBuilding` inside `ReceiveDamage` case 4 before later building destruction/removal work continues. | Rust snapshots event during combat death, despawns/handles death, then world drains `destroyed_garrison_buildings` after crew ejection handling. | UNCHECKED for tick/substage equality; player-visible no-exit result is same in the scoped case |

## Findings

### PASS-01: blocked destruction does not place occupants

For a destroyed 2x2 garrison with all 14 `SellBuilding` edge candidates blocked, native selects the no-exit null fallback and produces `0` successful ejections. Current Rust also returns `0` from `eject_destruction_garrison` and does not place occupants on any edge or foundation cell.

### PASS-02: no parachute fallback is correct

Native `SellBuilding` calls `SpawnUnitsWithParachute(0)`, but the zero argument selects the null destroy/remove branch. Current Rust correctly does not create parachute state or chute visuals for this scoped destruction fallback.

### PASS-03: no scatter, mission queue, or RNG is consumed

Native null fallback never reaches the successful-ejection `Unlimbo`/scatter/mission handoff and contains no RNG calls. Current Rust's blocked destruction branch only marks passengers removed and returns before placement/scatter code; the focused unit test asserts unchanged RNG state.

### UNCHECKED-01: exact occupant `+0xF8` removal bytes

The trace verifies that native calls each occupant's vtable `+0xF8` in reverse order, but does not decompile the occupant subclass implementation behind `+0xF8`. Rust keeps an entity record marked `health=0`, `dying=true`, and `PassengerRole::None`. Whether that is byte/state-identical to the native destroy/remove virtual remains unchecked.

### UNCHECKED-02: exact same-tick destruction substage

Native performs the null fallback inside `ReceiveDamage` case 4. Rust defers through a `DestroyedGarrisonBuilding` event drained by `Simulation::advance_tick`. The final scoped no-exit result matches, but exact substage timing/order was not numerically compared.

## Adjacent Findings

- Player sell is different: native normal player sell passes the nonzero second argument and uses the inside-foundation fallback when no edge cell is accepted. This trace is destruction-only.
- Successful edge ejection scatter is different scope: native calls the occupant scatter virtual after `Unlimbo`; this trace stops at the no-exit null branch where scatter is absent.
- Rust's `Can_Enter_Cell` predicate is still an approximation; in this all-blocked fixture the accepted candidate count is the same, but exact predicate parity is not proven.

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Status

COMPLETE for the requested blocked-exit destruction fallback. Exact occupant subclass `+0xF8` internals and same-tick substage ordering remain explicitly UNCHECKED, not PASS.
