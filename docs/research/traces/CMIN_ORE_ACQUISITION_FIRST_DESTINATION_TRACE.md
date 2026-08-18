# Chrono Miner Ore Acquisition First Destination Trace

**Scenario:** Standard YR Allied Chrono Miner (`CMIN`) in `Mission_Harvest` state 0 / Rust `SearchOre`, empty cargo, visible reachable Riparius ore in scan range. Trace only: scan, selected ore cell, destination assignment, locomotor decision, and first visible movement toward ore.

**Date:** 2026-05-23  
**Trace swarm slot:** 1  
**Scope:** First ore-acquisition destination only. Harvest density/cargo, depletion retarget, return-to-refinery, docking, dump/deposit, and post-return behavior are adjacent slots.

## Summary Verdict

For this scoped scenario, gamemd drives the Chrono Miner to the first selected ore cell; it does not start the chrono warp state machine for the outbound ore-acquisition move.

Current Rust also routes the same scenario to ground movement. `handle_search_ore` selects a resource node and sets `target_ore_cell`, then `handle_move_to_ore` issues `issue_move_if_idle` / `issue_direct_move`. It does not call `issue_teleport_command` for outbound ore acquisition.

The branch-level observable result therefore matches for the warp-vs-drive question: `drive_move=1`, `teleport_move=0`, immediate chrono warp/audio/effect count `0`. Exact selected ore-cell equality remains unchecked because this prompt did not provide a concrete map coordinate, ore-density layout, or competing ore cells.

## Sources Checked

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_SYSTEM_OVERVIEW.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/CHRONO_MINER_ORE_ACQUISITION_WARP_VS_DRIVE_SWARM_20260520_TRACE.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`
- `ini/rulesmd.ini:7351-7400`
- `src/sim/miner/miner_system.rs:327-505`
- `src/sim/movement/movement_commands.rs:63-150`
- `src/sim/movement/teleport_movement.rs:103-142`
- `src/sim/world/world_commands.rs:154-179`

Ghidra MCP was used read-only, but this session's batch decompile endpoint did not resolve the requested function addresses or names. No mutating Ghidra calls were made. The gamemd evidence below therefore relies on the verified research docs listed above, including prior read-only decompilation citations in those docs.

## Standard YR Data

Stock `[CMIN]` in `rulesmd.ini` has:

- `Harvester=yes`
- `Teleporter=yes`
- `Speed=4`
- `Storage=20`
- `ROT=5`
- `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` (teleport locomotor)
- `MovementZone=Crusher`

The ore type in this scenario is Riparius. The ore overlay report confirms stock YR Riparius ore is active tiberium/ore data, not dormant TS legacy. `OverlayTypeClass.Tiberium=yes`, `CellClass+0x11E` is density, and standard harvesters use the non-Weeder `UnitClass::Harvest_Ore_Tick` path.

## Pipeline

gamemd:

`UnitClass::Mission_Harvest state 0 -> TiberiumLongScan -> Search_For_Tiberium_And_Move -> selected reachable Riparius cell -> TechnoClass::Set_Destination / FootClass destination dispatch -> active Drive locomotor -> visible ground drive toward ore`

Rust:

`handle_search_ore -> search_local_ore / pick_best_resource_node -> target_ore_cell=Some(cell), state=MoveToOre -> handle_move_to_ore -> issue_move_if_idle or issue_direct_move -> MovementTarget -> visible ground movement toward ore`

## Stage Results

### Stage 1 - CMIN Type and Scenario Preconditions

gamemd output:

- `Harvester=1`
- `Teleporter=1`
- primary locomotor CLSID is teleport
- movement zone is Crusher
- empty cargo enters harvest search instead of return

Rust output:

- CMIN rule parsing exposes harvester + teleporter state and miner classification.
- `MinerConfig::from_general_rules` uses `TiberiumLongScan` for state-0 scan radius.

Verdict: **UNCHECKED**. The stock INI values are verified, but this trace did not run a Rust parser dump and compare every parsed field numerically.

### Stage 2 - State-0 Ore Scan and Selected Cell

gamemd output:

- `Mission_Harvest` state 0 uses `TiberiumLongScan` for both HARV and CMIN.
- The active path is standard YR harvester logic, not TS legacy.
- The scan returns a reachable ore destination when visible Riparius ore exists in range.

Rust output:

- `handle_search_ore` calls `search_local_ore(..., config.long_scan_radius, ...)`.
- If the bounded scan finds ore, Rust writes `target_ore_cell=Some(cell)` and `state=MoveToOre`.
- If no bounded cell is found, it falls back to `pick_best_resource_node`.

Verdict: **UNCHECKED**. Both sides scan for ore, but no concrete coordinate/density layout was supplied, so the exact selected ore cell cannot be numerically compared.

### Stage 3 - Destination Assignment and Warp-vs-Drive Decision

gamemd output:

- In the normal state-0 acquisition case, `Search_For_Tiberium_And_Move` drives the selected ore destination through the ground locomotor path.
- The bounded decision result for this scenario is `drive_move=1`, `teleport_move=0`.
- This is active standard YR CMIN harvester behavior per the May 20 acquisition trace and the corrected system overview; the older drive-phase trace contains a contradictory ore-cell conclusion and is not used as controlling evidence here.

Rust output:

- `handle_move_to_ore` checks for an already-active `teleport_state` and returns only if one exists.
- It never calls `issue_teleport_command` for outbound ore acquisition.
- For adjacent ore it calls `movement::issue_direct_move`.
- For non-adjacent ore it calls `issue_move_if_idle`, which attaches a ground `MovementTarget`.
- The bounded decision result is `drive_move=1`, `teleport_move=0`.

Verdict: **PASS**. The player-visible first movement mode matches: the miner drives to ore rather than warping.

### Stage 4 - First Visible Movement Object

gamemd output:

- The player sees ground movement toward the ore cell.
- Immediate chrono teleport visual/audio count for the outbound acquisition move is `0`.

Rust output:

- `issue_direct_move` creates a two-cell `MovementTarget`.
- `issue_move_command` / `issue_move_if_idle` creates a path-backed `MovementTarget`.
- `issue_teleport_command` would attach `TeleportState`, remove ground movement, and trigger the teleport path, but that call is absent from `handle_move_to_ore`.
- Immediate chrono teleport visual/audio count for this path is `0`.

Verdict: **PASS**. Both sides produce no immediate warp-out/warp-in for first ore acquisition.

### Stage 5 - Tick, Path, Facing, and Arrival Details

gamemd output:

- Drive locomotor pathing, facing, track progression, occupancy updates, and arrival timing determine the exact visible route and cadence after the branch decision.

Rust output:

- Rust uses A* or direct movement with fixed-point speed and movement targets.
- The final path cells, turn timing, lepton positions per tick, and arrival tick were not compared against gamemd for a concrete map.

Verdict: **UNCHECKED**. This slot only resolves the first-destination movement-mode branch; exact drive parity belongs to a movement/path trace with concrete coordinates.

## Failures

None for the scoped warp-vs-drive decision.

## Not Implemented

None for the scoped warp-vs-drive decision.

## Unchecked Risk

- Exact selected ore cell parity is unchecked without a concrete map/ore-density layout.
- Exact path and first-step timing are unchecked: A* cells, drive-track selection, facing progression, and lepton positions were not numerically compared.
- Rust does not model the original COM `IPiggyback` object graph literally. That is not a scoped failure because the observable branch result here is drive/no-warp, but it remains relevant to later return/dock traces.

## Adjacent Findings

- `CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md` argues that ore acquisition warps, but it also records an internal contradiction about ore cells versus the later corrected acquisition trace. This report uses the May 20 acquisition-specific trace as controlling evidence.
- Broader scan-order parity, ore density selection, retargeting after depletion, cargo loading, return teleport threshold, and refinery dump/deposit are adjacent mechanics and were not traced here.

## Verdict Tally

PASS: 2 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0
