# MCV Undeploy Move-Out Occupancy Post-Fix Trace

**Scenario:** Allied `AMCV` deploys into `GACNST`; `GACNST` undeploys back into `AMCV`; the newly spawned `AMCV` receives a move order and leaves the old ConYard footprint.
**Date:** 2026-05-26
**Status:** post-fix verification trace.
**Overall verdict:** PASS for the player-visible stuck-MCV bug. UNCHECKED for exact gamemd hidden-occupancy counter breadth and the broader ConYard redeploy state machine.

## Pipeline

1. `AMCV` deploy command.
2. Rust spawns `GACNST` and registers structure occupancy.
3. `GACNST` undeploy command enters reverse build-down.
4. Build-down completion removes old `GACNST` occupancy, despawns the building, and spawns `AMCV`.
5. Player move command attaches a movement target to the new `AMCV`.
6. Movement tick lets `AMCV` leave the old footprint.

## Stage Results

### 1 - Data Keys

Input:
- `rulesmd.ini [AMCV] DeploysInto=GACNST`, `Speed=4`, `ROT=5`, `Strength=1000`.
- `rulesmd.ini [GACNST] ConstructionYard=yes`, `UndeploysInto=AMCV`, `Strength=1000`.
- `artmd.ini [GACNST] Foundation=4x4`.
- `rulesmd.ini [MultiplayerDialogSettings] MCVRedeploys=yes`.

gamemd:
- Existing MCV deployment reports verify these keys drive the stock YR MCV/ConYard conversion path.

Rust:
- `deploy_mcv` resolves `DeploysInto`.
- `undeploy_building` resolves `UndeploysInto`.
- `spawn_object_at_height` reads the parsed foundation.

Verdict: PASS for scoped data lookup.

### 2 - gamemd Building Occupancy Removal

gamemd:
- Fresh read-only Ghidra spot-check of `BuildingClass::Limbo @ 0x00445880` confirms the active removal path:
  - Computes foundation width and height.
  - Gets the building origin/deploy cell through vtable `+0x1B8`.
  - Iterates `(width + 2) * (height + 2)` cells from one cell before that origin.
  - Decrements `CellClass+0x122` for each visited cell.
- For stock `GACNST Foundation=4x4`, this removes a 6x6 occupancy-counter region.

Rust:
- Rust does not model `CellClass+0x122` as the same hidden counter.
- Rust movement blockers use `OccupancyGrid`, where structures are registered on their base foundation cells.

Verdict: UNCHECKED for full hidden-counter parity, PASS for the fact that the old building must clear its movement-blocking footprint before the replacement MCV moves.

### 3 - Rust Building Occupancy Removal After Fix

Rust before fix:
- `tick_building_down` called `despawn_entity(sid)` directly.
- `despawn_entity` removed only the origin cell from `self.occupancy`.
- Result: old ConYard foundation cells remained as ghost blockers.

Rust after fix:
- `tick_building_down` now looks up the structure object and calls `production::building_base_foundation_cells(entity.position.rx, entity.position.ry, &obj.foundation)`.
- It removes `sid` from every returned foundation cell before calling `despawn_entity(sid)`.
- `despawn_entity` still removes the origin cell again; `OccupancyGrid::remove` is idempotent, so this is harmless.

Concrete scoped output:
- For a Rust 4x4 ConYard foundation occupancy, all 16 movement-blocking foundation cells are removed before the new AMCV is spawned.
- No stale structure blocker remains in the base foundation cells used by Rust movement checks.

Verdict: PASS for the scoped Rust blocker cleanup needed for the stuck-MCV symptom.

### 4 - Spawned AMCV Commandability

Rust:
- `tick_building_down` spawns the replacement `AMCV` through `spawn_object_at_height`.
- Selection transfers from the old ConYard.
- The new `AMCV` has no `deploy_state`; `Command::Move` does not reject it via `is_deployed()`.

gamemd:
- Existing MCV report verifies the reverse conversion creates the MCV and destroys the source building. No evidence indicates the created MCV remains movement-locked after successful conversion.

Verdict: PASS for the narrow move-commandability question.

### 5 - End-To-End Movement Result

Verification command:

```text
cargo test --lib test_undeploy_conyard_spawns_mcv -- --nocapture
```

Observed:
- Test deploys AMCV to GACNST.
- Clears construction-up state, undeploys the GACNST, and advances through build-down completion.
- Finds the spawned AMCV at `(21, 22)`.
- Issues `Move` to `(27, 22)`.
- Advances movement ticks.
- Asserts the AMCV no longer remains at `(21, 22)`.

Result:
- `test sim::world::tests::test_undeploy_conyard_spawns_mcv ... ok`
- `1 passed; 0 failed`.

Verdict: PASS for the player-visible regression.

## Remaining Drift / Not Covered

- Rust still uses a simplified 30-tick `BuildingDown` rather than the exact gamemd ConYard redeploy mission machinery.
- Rust still does not enforce every gamemd ConYard redeploy gate (`MCVRedeploys`, MP/human-control, power link, production-busy button visibility) in this traced path.
- Rust does not model gamemd's full `(foundation + 2)` hidden occupancy counter region. This trace only proves the movement-blocking base foundation cells are now cleared.

## Verdict Tally

- PASS: 3
- FAIL: 0 for the scoped stuck-MCV fix
- UNCHECKED: 2
- NOT IMPLEMENTED: 0 in the scoped movement unblock path

## Evidence

- Rust source:
  - `src/sim/world/mod.rs::tick_building_down`
  - `src/sim/world/world_tests.rs::test_undeploy_conyard_spawns_mcv`
  - `src/sim/world/world_spawn.rs::spawn_object_at_height`
- Test:
  - `cargo test --lib test_undeploy_conyard_spawns_mcv -- --nocapture`
- Ghidra read-only:
  - `BuildingClass::Limbo @ 0x00445880`
- Existing research:
  - `docs/research/traces/MCV_UNDEPLOY_MOVE_OUT_OCCUPANCY_TRACE_2026-05-26.md`
  - `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md`
  - `docs/research/MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`
