# Chrono Miner Ore Acquisition Warp-vs-Drive Trace

**Scenario:** Standard YR Allied Chrono Miner (`CMIN`) in harvest/search flow selects a reachable ore cell away from refinery flow. Verify whether `gamemd.exe` uses chrono teleport or drive locomotion to approach the ore, and compare against the Rust movement command path.

**Date:** 2026-05-20  
**Trace swarm slot:** 1  
**Scope:** Ore acquisition movement only. Return-to-refinery, dock handoff, forced return, CloseEnough, and post-dump exit are adjacent swarm slots.  
**Write constraint:** This is the only file written for this slot.

## Summary Verdict

For this concrete scenario, `gamemd.exe` drives the Chrono Miner to the selected ore cell. It does not initiate the chrono warp state machine for the state-0 ore-acquisition move.

The critical discriminator is not only `CMIN Teleporter=yes`. In `UnitClass::Mission_Harvest` state 0, `FootClass::Search_For_Tiberium_And_Move` only issues a new destination when `FootClass+0x5A4` is null. In `TechnoClass::Set_Destination`, that null previous-destination case goes through the teleporter block path that creates/activates `DriveLocomotionClass`, then `FootClass::Set_Destination_Internal` dispatches `Head_To_Coord` to the active Drive locomotor. The teleport locomotor's `Head_To_Coord` path, which would set the teleport moving flag, is not the one used for this ore-acquisition move.

Current Rust also routes this scenario to ground movement, not teleport: `handle_search_ore` stores `target_ore_cell`, `handle_move_to_ore` waits out any existing teleport but never calls `issue_teleport_command`, and then issues either A* movement or a direct final-step movement target.

Verdict tally: PASS: 2 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Sources Checked

- `ini/rulesmd.ini:7351-7400`: stock `[CMIN]` has `Harvester=yes`, `Teleporter=yes`, `Speed=4`, `ROT=5`, `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Crusher`.
- `docs/research/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`: state 0 is active in standard YR for both HARV and CMIN; it uses `TiberiumLongScan=48`, calls `FootClass::Search_For_Tiberium_And_Move @ 0x4DCFE0`, and has no separate chrono scan function.
- Ghidra read-only spot checks:
  - `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0`
  - `FootClass::Scan_For_Tiberium @ 0x004DD0A0`
  - `TechnoClass::Set_Destination @ 0x00741970`
  - `FootClass::Set_Destination_Internal @ 0x004D94B0`
  - `TeleportLocomotionClass::Head_To_Coord @ 0x00718100`
- Current Rust source:
  - `src/sim/miner/miner_system.rs:341-365`
  - `src/sim/miner/miner_system.rs:375-480`
  - `src/sim/world/world_commands.rs:899-923`
  - `src/sim/movement/movement_commands.rs:55-82`
  - `src/sim/movement/movement_commands.rs:98-145`
  - `src/sim/movement/movement_commands.rs:359-383`

## Pipeline

`Mission_Harvest state 0 -> scan nearest reachable ore ring -> Search_For_Tiberium_And_Move -> TechnoClass::Set_Destination(ore CellClass, flag=1) -> DriveLocomotion active -> FootClass::Set_Destination_Internal -> Drive Head_To_Coord/pathing -> visible ground drive toward ore`

Rust:

`handle_search_ore -> target_ore_cell=Some(cell), state=MoveToOre -> handle_move_to_ore -> no issue_teleport_command -> issue_move_if_idle/issue_direct_move -> MovementTarget -> movement_tick ground movement`

## Stage Results

### Stage 1 - Stock CMIN movement flags

**gamemd output:** `Teleporter=1`, `Harvester=1`, primary locomotor CLSID = Teleport, `MovementZone=Crusher`, `Speed=4`.

**Rust output:** rule parsing and miner classification treat `Harvester=yes + Teleporter=yes` as `MinerKind::Chrono`; the movement path has chrono-specific handling.

**Verdict:** UNCHECKED. The INI data is verified, but this slot did not run a parser-value dump from Rust and compare each parsed field numerically.

### Stage 2 - State-0 ore scan and target selection

**gamemd output:** state 0 uses `TiberiumLongScan=48`; the scan radius is converted to cells and the scan calls `FootClass::Search_For_Tiberium_And_Move`. The path is active in standard YR for both HARV and CMIN.

**Rust output:** `handle_search_ore` uses `config.long_scan_radius` and records `target_ore_cell`, then transitions to `MoveToOre`.

**Verdict:** UNCHECKED. The broad radius source matches, but the exact selected ore cell for this scenario was not numerically instantiated with a concrete map/ore-density layout in both engines.

### Stage 3 - Warp-vs-drive decision after reachable ore is selected

**gamemd output:** for the normal state-0 acquisition case, previous destination is null (`FootClass+0x5A4 == 0`) before `Search_For_Tiberium_And_Move` calls `Set_Destination`. In `TechnoClass::Set_Destination`, that case enters the teleporter/piggyback path that creates or activates `DriveLocomotionClass`. `FootClass::Set_Destination_Internal` then calls `Head_To_Coord` on the active locomotor. Because active locomotor is Drive for this path, the teleport `Head_To_Coord` flag set is not executed.

Numerical branch result for this scoped decision: `drive_move=1`, `teleport_move=0`.

**Rust output:** `handle_move_to_ore` has a guard that returns only when an existing teleport is already in progress, then it issues ground movement. The function does not call `issue_teleport_command`. When the target is adjacent it calls `movement::issue_direct_move`; otherwise, with a `PathGrid`, it calls `issue_move_if_idle`, which delegates to `issue_move_command`.

Numerical branch result from current code path: `drive_move=1`, `teleport_move=0`.

**Verdict:** PASS. The scoped branch decision matches: selected reachable ore is approached by ground movement, not chrono teleport.

### Stage 4 - Rust movement command attachment

**gamemd output:** active Drive locomotor receives the destination and runs normal drive/pathing.

**Rust output:** `issue_move_command` computes an A* path and attaches `MovementTarget`; `issue_direct_move` attaches a two-cell `MovementTarget` for the final ore step.

**Verdict:** UNCHECKED. Both sides use ground movement, but this slot did not compare exact A* path cells, turn-track selection, facing progression, or tick-by-tick lepton positions for a concrete map.

### Stage 5 - Player-visible teleport absence

**gamemd output:** because the teleport `Head_To_Coord` path is not used for this decision, the chrono warp state machine is not armed for the ore-acquisition approach. Expected immediate chrono warp/audio count for this scoped outbound ore move: `0`.

**Rust output:** no `issue_teleport_command` call is made by `handle_move_to_ore`; `teleport_state` is not created for the ore-acquisition move. Expected immediate chrono warp/audio count from this path: `0`.

**Verdict:** PASS. The player should not see the miner vanish/reappear or hear chrono teleport audio when it selects and approaches ore in this state-0 acquisition path.

## Failures

None in this scoped warp-vs-drive decision.

## Not Implemented

None in this scoped warp-vs-drive decision.

## Unchecked Risk

- Exact ore-cell selection remains unchecked for this scenario because no concrete ore-density layout was provided and this slot did not instantiate one in both engines.
- Exact movement parity after the drive decision is unchecked: path cells, drive-track timing, rotation, acceleration/deceleration, stuck handling, and final ore-cell entry were not numerically compared in this slot.
- Rust does not model the original COM `IPiggyback` object graph literally. That is not a scoped failure here because the observable branch result is drive/no-warp, but it remains a risk for exact tick/path/facing parity.

## Adjacent Findings

- `CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md` contains an internal contradiction around ore cells. This slot resolves the bounded state-0 ore-acquisition case as drive/no-warp by checking `Search_For_Tiberium_And_Move` and the null previous-destination branch in `TechnoClass::Set_Destination`.
- `MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md` already reports broader scan-selection mismatches between gamemd and Rust. Those can change which ore cell is selected, but they are adjacent to the warp-vs-drive decision and were not expanded here.
