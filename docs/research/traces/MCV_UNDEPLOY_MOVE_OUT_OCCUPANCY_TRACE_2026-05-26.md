# MCV Undeploy Move-Out Occupancy Trace

**Scenario:** retail Allied `AMCV` deploys into `GACNST`, `GACNST` undeploys back into `AMCV`, then the player issues a move order to the newly spawned `AMCV`.
**Date:** 2026-05-26
**Verdict:** FAIL in Rust occupancy cleanup after ConYard undeploy. gamemd removes the building occupancy before/while the MCV replacement is placed; Rust leaves stale ConYard blockers in most foundation cells.

## Pipeline

1. Player deploy command on selected `AMCV`.
2. `AMCV` converts to `GACNST`.
3. Player undeploy command on selected `GACNST`.
4. `GACNST` reverse conversion completes and spawns an `AMCV`.
5. Player move command targets the new `AMCV`.
6. Movement/pathing checks occupancy for the route out of the old foundation.

## Stage Results

### 1 - INI Data

Input:
- `ini/rulesmd.ini [AMCV] DeploysInto=GACNST`, `Speed=4`, `ROT=5`, `Strength=1000`.
- `ini/rulesmd.ini [GACNST] ConstructionYard=yes`, `UndeploysInto=AMCV`, `Strength=1000`.
- `ini/artmd.ini [GACNST] Foundation=4x4`.
- `ini/rulesmd.ini [MultiplayerDialogSettings] MCVRedeploys=yes`.

gamemd:
- Existing reports verify these keys drive the stock YR bidirectional MCV/ConYard path.

Rust:
- `deploy_mcv` resolves `DeploysInto`.
- `undeploy_building` resolves `UndeploysInto`.
- `spawn_object_at_height` reads the parsed foundation and registers structure occupancy on every base foundation cell.

Verdict: PASS for key lookup. Rust does not yet enforce every gamemd ConYard redeploy gate, but that is not the blocker for this scenario.

### 2 - Forward Deploy Occupancy

gamemd:
- `UnitClass::Deploy @ 0x007393C0` creates the target building and places it through `BuildingClass::Unlimbo @ 0x00440580`.
- Fresh Ghidra check of `BuildingClass::Unlimbo @ 0x00440580` shows it computes foundation width/height and increments `CellClass+0x122` over `(width + 2) * (height + 2)` cells around the building origin.
- For retail `GACNST Foundation=4x4`, this is a 6x6 counter region.

Rust:
- `Simulation::deploy_mcv` despawns the AMCV, then calls `spawn_object_at_height`.
- `spawn_object_at_height` registers `GACNST` in `self.occupancy` for the 4x4 base foundation cells via `building_base_foundation_cells`.

Verdict: UNCHECKED for exact hidden-occupancy breadth because Rust tracks only base foundation cells, while gamemd also updates a surrounding counter region. For the base cells used by movement blockers, Rust does register the ConYard.

### 3 - Reverse Conversion Trigger

gamemd:
- Existing reports verify the ConYard path: UI/runtime gates pass only in MP/skirmish-style mode with human owner, `MCVRedeploys` enabled, and no power link.
- On success, `BuildingClass::Mission_Deploy @ 0x0073D630` / helper path reaches the shared conversion path and creates an MCV unit.

Rust:
- `Command::UndeployBuilding` calls `undeploy_building` after ownership validation.
- `undeploy_building` sets `BuildingDown { total_ticks: 30, spawn_type: AMCV, spawn_rx/spawn_ry = foundation center }`.

Verdict: FAIL/DRIFT for gate/timing mechanism. Rust uses a simplified 30-tick `BuildingDown` and lacks several gamemd gates. This is adjacent to, but not the root cause of, the move-out failure.

### 4 - Building Removal Occupancy

gamemd:
- Fresh Ghidra check of `BuildingClass::Limbo @ 0x00445880` shows the removal path mirrors placement: it computes foundation width/height, gets the deploy/origin cell through vtable `+0x1b8`, then decrements `CellClass+0x122` over `(width + 2) * (height + 2)` cells.
- For retail `GACNST Foundation=4x4`, the 6x6 occupancy-counter region touched by `Unlimbo` is cleaned by `Limbo`.
- This means the old ConYard footprint is not left as a blocker for the newly created MCV's first move.

Rust:
- `tick_building_down` calls `self.despawn_entity(sid)` when the reverse build-up completes.
- `despawn_entity` removes only `(entity.position.rx, entity.position.ry)` from `self.occupancy`.
- The function comment explicitly says multi-cell structures should have their foundation cells removed by the caller before `despawn_entity`, but `tick_building_down` does not do that.

Concrete Rust output:
- For a 4x4 ConYard registered on 16 base foundation cells, undeploy completion removes 1 cell and leaves up to 15 stale building blocker cells.
- The spawned AMCV is placed at the foundation center, which is inside the stale footprint.

Verdict: FAIL. This is the direct parity mismatch causing the new MCV to be stuck or unable to move out normally.

### 5 - New MCV Selection And Move Command

gamemd:
- Existing MCV deploy report says the reverse conversion creates the MCV unit and destroys the source building, transferring state.
- No evidence found that gamemd leaves the new unit in a deployed movement-blocked state after successful conversion.

Rust:
- `tick_building_down` spawns `AMCV` through `spawn_object_at_height` and preserves `selected`.
- The new AMCV has no `deploy_state`; `Command::Move` only rejects entities where `is_deployed()` is true.
- Therefore the command gate is not the blocker.

Verdict: PASS for the narrow question "is the new MCV still marked deployed?" It is not. The movement failure comes after command acceptance, from stale occupancy.

### 6 - Player-Visible Result

gamemd:
- After successful ConYard redeploy, the replacement MCV can be ordered away from the old yard position because the old building's occupancy is removed.

Rust:
- The new AMCV is selected and visible, but path/step checks see stale ConYard blockers in the old foundation cells.
- Player-visible symptom: after undeploying, right-click move orders appear to do nothing or the MCV remains trapped at the old ConYard position.

Verdict: FAIL.

## Failures

1. **HIGH - stale ConYard occupancy after undeploy**
   - Rust `tick_building_down` removes only the structure origin cell through `despawn_entity`.
   - gamemd `BuildingClass::Limbo` decrements the whole building occupancy area.
   - Player-visible result: the undeployed MCV cannot move out normally.

2. **MEDIUM - reverse conversion timing/gates are simplified**
   - Rust uses a generic 30-tick `BuildingDown`.
   - gamemd uses the BuildingClass deploy/redeploy mission machinery and ConYard-specific gates.
   - This should be handled separately from the immediate movement blocker bug.

## Rust Touchpoints

- `src/sim/world/mod.rs::tick_building_down`
- `src/sim/world/mod.rs::despawn_entity`
- `src/sim/world/world_spawn.rs::spawn_object_at_height`
- `src/sim/world/world_spawn.rs::undeploy_building`
- `src/sim/world/world_commands.rs::Command::Move`

## Evidence

- Ghidra read-only decompile: `BuildingClass::Unlimbo @ 0x00440580`.
- Ghidra read-only decompile: `BuildingClass::Limbo @ 0x00445880`.
- Existing docs: `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md`, `docs/research/MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`, `docs/research/GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`, `ini/artmd.ini`.
- Rust source scan listed in touchpoints above.
