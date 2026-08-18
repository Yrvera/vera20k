# Garrison Destroyed Blocked No-Fallback Postfix Trace

**Scenario:** an occupied civilian `CanBeOccupied` garrison is destroyed while every SellBuilding-style edge exit cell is blocked.

**Concrete fixture:** stock YR `CAEUR1` as the representative occupied civilian garrison. `rulesmd.ini:18460..18480` keeps `CanBeOccupied=yes` and `MaxNumberOccupants=3`; `artmd.ini:6063..6064` gives `Foundation=2x2`. For a 2x2 foundation at `(10,10)`, the verified SellBuilding perimeter scan has 14 candidate probes.

**Scope:** destruction no-exit fallback only. Player sell inside-foundation fallback, successful edge ejection scatter, generic transport unload, and non-null parachute helper callers are adjacent findings only.

## Sources Checked

- Read-only Ghidra spot-check: `BuildingClass::ReceiveDamage @ 0x00442230`, `BuildingClass::SellBuilding @ 0x00457DE0`, `SpawnUnitsWithParachute @ 0x004585C0`.
- Verified docs: `docs/research/GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`, `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`, `docs/research/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`.
- Current Rust scan: `src/sim/combat/mod.rs:860..899`, `src/sim/world/mod.rs:1470..1473`, `src/sim/production/production_sell.rs:243..460`, `src/sim/production/production_sell.rs:522..545`, `src/sim/production/production_sell.rs:1125..1157`.

No Cargo command was run in this trace: the hard constraint allowed writing exactly this report file, and Cargo would update build artifacts outside the allowed path.

## Pipeline

`ReceiveDamage death case` -> `CanBeOccupied gate` -> `SellBuilding edge probe using occupant slot 0` -> `0 accepted edge cells` -> `SpawnUnitsWithParachute(0)` null branch -> reverse occupant destroy/remove virtuals -> garrison vector cleanup -> no unlimbo, no scatter, no mission, no RNG, no parachute visuals.

Current Rust:

`combat death loop` -> snapshot garrison cargo -> `DestroyedGarrisonBuilding` event -> world drains event -> `eject_destruction_garrison` -> same edge helper -> no accepted edge cell -> reverse occupant `health=0`, `dying=true`, `PassengerRole::None` -> return `0` ejected -> no placement/scatter/RNG/parachute path.

## Stage Trace

| Stage | gamemd output | Rust output | Verdict |
|---|---:|---:|---|
| Live YR entry gate | `ReceiveDamage` case 4 checks `BuildingType+0x157B`; stock YR has active `CanBeOccupied=yes` civilian buildings including `CAEUR1` | Rust checks `obj.can_be_occupied`, `Structure`, and non-empty passengers before emitting `DestroyedGarrisonBuilding` | PASS |
| 2x2 edge scan cardinality | 14 probes for origin `(10,10)`, width `2`, height `2` | `garrison_sellbuilding_exit_cells(10,10,2,2)` builds 14 probes | PASS |
| Blocked-exit accepted count | all 14 probes rejected, accepted count `0` | all 14 blocked cells make `choose_garrison_exit_cell` return `None`; accepted count `0` | PASS |
| Destruction no-exit selector | `SellBuilding` calls `SpawnUnitsWithParachute(0)` and returns before normal unlimbo loop | `GarrisonEjectMode::DestructionNoExitRemove` maps no exit to `None` | PASS |
| Successful ejections | null branch unlimbos `0` occupants | Rust returns `0` from `eject_destruction_garrison` | PASS |
| Occupant order | null branch starts at occupant count minus one and decrements to `0` | Rust iterates `passenger_ids.iter().rev()` | PASS |
| No placement fallback | no `Unlimbo`, no inside-foundation fallback, no edge/foundation position assignment | no `place_garrison_passenger_at_cell` call when `exit_cell == None` | PASS |
| RNG consumption | `0` RNG calls in the null branch | `mark_garrison_passenger_removed` performs `0` RNG calls; focused test asserts unchanged RNG state | PASS |
| Scatter and mission | no vtable `+0x174` Scatter and no `+0x1E8` mission queue | branch returns before `sellbuilding_direct_scatter_handoff`; no mission write in no-exit branch | PASS |
| Parachute visuals/state | no `ObjectClass::Unlimbo`, no falling state, no `PARACH`, no chute sound | no `parachute_state` or chute helper is touched by `eject_destruction_garrison` | PASS |
| Cargo/vector cleanup | native clears the building garrison vector after occupant removals | Rust building has already been removed; no surviving building cargo remains, but byte-level vector cleanup equivalence is not applicable | UNCHECKED |
| Occupant destroy bytes | native calls each occupant vtable `+0xF8` | Rust keeps an entity record marked dead/dying with no passenger role | UNCHECKED |
| Same-tick substage | native executes inside `ReceiveDamage` case 4 before later building removal/destruction continuation | Rust snapshots in combat and drains the event later in `Simulation::advance_tick` after crew-survivor handling | UNCHECKED |

## Findings

### PASS-01: no fallback placement occurs

When all 14 edge probes are blocked, gamemd reaches the `SpawnUnitsWithParachute(0)` null branch and performs zero unlimbos. Current Rust selects `None` for destruction no-exit and returns `0` successful ejections without placing occupants on an edge or inside the foundation.

### PASS-02: no RNG or Scatter is consumed

The native null branch has no `RandomRanged`, no `Random::Next`, no Scatter virtual, and no mission queue call. Current Rust's no-exit branch only marks passengers removed and returns before the placement/scatter handoff; the focused unit fixture records unchanged RNG state.

### PASS-03: no parachute behavior is triggered

Despite the helper name, `SpawnUnitsWithParachute(0)` takes the null branch. No parachute anim, falling state, landing mission, or chute sound is created. Current Rust also does not invoke parachute state/render paths for destroyed blocked garrisons.

### UNCHECKED-01: exact occupant removal bytes

The native branch calls each occupant's vtable `+0xF8`; this trace did not bind every subclass implementation behind that virtual. Rust's dead/dying entity state matches the visible removal result, but byte/state identity remains unchecked.

### UNCHECKED-02: exact destruction substage

Native performs the fallback inside `ReceiveDamage` case 4. Rust emits and drains a `DestroyedGarrisonBuilding` event during the same simulation tick, but this trace did not numerically prove substage equality against every adjacent death side effect.

## Adjacent Findings

- Player sell no-exit is different: normal player sell uses a non-null fallback path and may use an inside-foundation coordinate. This report is destruction-only.
- Successful edge ejection remains a separate Scatter trace. This no-exit branch never reaches Scatter.
- Exact `Can_Enter_Cell` predicate parity remains broader than this all-blocked fixture; here the concrete accepted count is still `0 == 0`.

## Verdict Tally

PASS: 10 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Status

COMPLETE for the requested destroyed blocked-exit no-fallback scenario. No player-visible FAIL or NOT-IMPLEMENTED item was found; byte-level occupant destroy internals and exact substage ordering remain UNCHECKED.
