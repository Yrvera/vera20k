# Chrono Miner Forced Return Command Movement State Swarm Trace

**Date:** 2026-05-20  
**Scenario:** Player explicitly orders a loaded or partially loaded Allied Chrono Miner (`CMIN`) at about `(40,40)` to return/unload at a clicked standard Allied refinery (`GAREFN`) near `(10,10)`.  
**Scope:** clicked-refinery preservation, first mission/movement state, first visible movement, and comparison to current Rust.  
**Out of scope:** ore search, post-dump exit, sounds, cursor frame ids, and unload animation/facing details beyond the first movement state.

## Sources

- Current Rust source under `C:/Users/enok/Documents/ra2-rust-game/src/`.
- Retail INI data in `ini/rulesmd.ini`, `ini/artmd.ini`, with base files as fallback.
- Prior context trace: `C:/Users/enok/Documents/ra2-rust-game-docs/traces/chrono_miner_forced_return_unload_command_TRACE.md`.
- Read-only Ghidra MCP decompile of active standard-YR functions:
  - `UnitClass__Mission_Harvest @ 0x0073E5E0`.
  - `TechnoClass__Set_Destination @ 0x00741970`.
  - `FootClass__Find_Docking_Bay @ 0x004DF040`.
  - `UnitClass__Receive_Radio @ 0x00737430`.
- Research docs:
  - `CHRONO_MINER_SYSTEM_OVERVIEW.md`.
  - `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`.
  - `MINER_MANUAL_ORDER_PARTIAL_CARGO_TO_REFINERY_TRACE.md`.

All Ghidra usage in this run was read-only. No labels, comments, symbols, structs, or program state were modified.

## Pipeline

Right-click friendly refinery -> app queues `Command::MinerReturn` with clicked refinery id -> command application writes `reserved_refinery` and enters `ForcedReturn` -> miner forced-return handler delegates to return handler -> chrono-return helper decides teleport-vs-drive -> movement/teleport system produces the first visible movement -> later dock/unload handshake.

## Concrete Data

- `CMIN` YR rules: `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=20`, `Teleporter=yes`.
- `GAREFN` YR rules/art: `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `Foundation=4x3`, `QueueingCell=4,1`.
- General rule: `ChronoHarvTooFarDistance=50`.
- Scenario cells used for numeric comparison:
  - Miner start: `(40,40)`.
  - GAREFN anchor: `(10,10)`.
  - Rust can-dock queue/dock anchor: `(13,11)` from `refinery_can_dock_queue_cell(rx, ry) = (rx + 3, ry + 1)`.
  - Rust staging cell without path-grid displacement: `(14,11)` from art `QueueingCell=4,1`.
  - Approximate building-center distance used for gamemd close-return check: `sqrt((40-12)^2 + (40-11)^2) * 256 = sqrt(1625) * 256 ~= 10318 leptons`.
  - gamemd chrono-return threshold: `50 * 256 = 12800 leptons`.

## Stage Table

| Stage | Boundary Checked | Rust Output | gamemd.exe Output | Verdict |
|---|---|---|---|---|
| 1 | Scenario data loaded | Rust rules/art expose CMIN miner/teleporter and GAREFN refinery/dock data from INI. | Retail INI has the same key data; active YR harvest code reads type flags such as `Harvester` and `Teleporter`. | UNCHECKED |
| 2 | Clicked refinery preserved | `app_context_order.rs:128-136` creates `Command::MinerReturn { target_refinery_id: clicked_friendly_refinery_id }`; `world_commands.rs:651-688` validates and stores it as `miner.reserved_refinery`. | Prior trace verified `FootClass__ClickedAction_Object @ 0x004D74E0` case `0x1A` passes the clicked object pointer into the order call. | UNCHECKED |
| 3 | First Rust miner state | `world_commands.rs:694-697` sets `forced_return=true`, `state=ForcedReturn`, and clears `movement_target`. | The exact order-to-mission setter for this manual command was not re-decoded to a literal mission byte in this run. | UNCHECKED |
| 4 | Forced return enters return handler | `handle_forced_return` keeps an explicit reservation when present and immediately calls `handle_return`. | gamemd active path for return/unload is `UnitClass__Mission_Harvest` state 2 plus refinery radio/dock machinery; exact manual-command state transition timing remains not fully decoded. | UNCHECKED |
| 5 | Teleport-vs-drive threshold | Rust compares squared cell distance from `(40,40)` to `(13,11)`: `27^2 + 29^2 = 1570`; threshold is `2^2 = 4`; `1570 > 4`, so Rust issues teleport. | gamemd state 2 for Teleporter harvesters compares distance against `RulesClass+0xD7C * 256 = 12800`; for this scenario approximate center distance is `~10318 <= 12800`, so it takes the close refinery radio path, not the far teleport-destination branch. | FAIL |
| 6 | First visible movement | `try_issue_chrono_return_teleport` calls `issue_teleport_command`; `teleport_movement.rs:183-190` relocates the unit to the staging cell in `TeleportPhase::Relocate`. | The active YR close path does not relocate the CMIN immediately; it negotiates docking/radio and uses drive/enter movement toward the refinery. | FAIL |
| 7 | First Rust visible destination | With standard GAREFN art and no closer passable replacement, Rust staging is `(14,11)` from `QueueingCell=4,1`; the unit snaps there on teleport tick. | In gamemd for this coordinate, no snap destination is produced because the close threshold branch is taken. | FAIL |
| 8 | Chrono lock after Rust warp | `issue_teleport_command(..., is_harvester=true)` sets `being_warped_ticks=0`; the relocation cleans up in one tick. | gamemd has no warp/chrono-lock phase for this scenario because it does not enter the far teleport branch. | FAIL |
| 9 | Dock entry after first movement | Rust will subsequently drive/enter/dock from its staging/contact flow, but exact tick-by-tick dock equality was not recomputed after the forced warp. | gamemd active dock arrival path is documented in `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`: Mission Enter, radio `0x15`, locomotor `Power_Off`, then unload. Literal tick equality for this scenario was not computed. | UNCHECKED |
| 10 | Unload activation | Rust dock sequence eventually enters `RefineryDockPhase::Unloading` after link/pivot logic. | gamemd sets Mission Unload through active standard-YR refinery radio and Mission Deploy/Building dump logic. Literal equality was not computed for this run. | UNCHECKED |

## Findings

### F1 - Forced Return Teleports When Retail Would Drive

**Stage:** 5  
**Verdict:** FAIL  
**Our code:** `src/sim/miner/miner_system.rs:36-39`, `src/sim/miner/miner_system.rs:852-890`.  
**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0` state 2 uses the active standard-YR Teleporter branch and compares return distance against `RulesClass+0xD7C * 0x100`; `rulesmd.ini:294` sets `ChronoHarvTooFarDistance=50`.

For `(40,40)` to a `GAREFN` near `(10,10)`, Rust uses a hardcoded 2-cell threshold from the Rust dock anchor `(13,11)`: `27^2 + 29^2 = 1570 > 4`, so it issues a teleport. gamemd's active YR chrono-harvester return branch treats the refinery as close enough: `~10318 <= 12800` leptons, so it uses the close refinery radio/drive path.

**Player-visible difference:** the chrono miner vanishes and reappears near the refinery instead of visibly driving back.

### F2 - First Movement State Skips Retail Radio/Drive Negotiation

**Stage:** 6  
**Verdict:** FAIL  
**Our code:** `src/sim/world/world_commands.rs:694-697`, `src/sim/miner/miner_system.rs:675-710`, `src/sim/miner/miner_system.rs:641-642`.  
**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0` state 2 calls the refinery docking/radio path when within the 50-cell chrono-harvester threshold; `UnitClass__Receive_Radio @ 0x00737430` contains active standard-YR docking cases `0x0E`, `0x15`, and `0x16`.

Rust enters `ForcedReturn`, delegates to `handle_return`, then immediately calls the chrono teleport helper when not already moving. For this coordinate, gamemd stays in the close return/docking path first.

**Player-visible difference:** Rust resolves the command as an instant special movement instead of a normal return-to-refinery drive/enter sequence.

### F3 - First Visible Position Changes Immediately In Rust

**Stage:** 7  
**Verdict:** FAIL  
**Our code:** `src/sim/miner/miner_system.rs:1030-1058`, `src/sim/miner/miner_dock_sequence.rs:70-81`, `src/sim/movement/teleport_movement.rs:183-190`.  
**gamemd evidence:** same active state-2 threshold branch in `UnitClass__Mission_Harvest @ 0x0073E5E0`; the close branch does not compute or assign the far teleport staging destination.

Rust computes a staging cell from GAREFN `QueueingCell=4,1`, normally `(14,11)`, then `TeleportPhase::Relocate` writes the entity position to the target cell. gamemd for this scenario does not create a first visible snap position at all.

**Player-visible difference:** the miner appears near the refinery too early and skips the trip from `(40,40)`.

### F4 - Rust Removes Chrono Lock Because It Treats This As Harvester Instant Warp

**Stage:** 8  
**Verdict:** FAIL  
**Our code:** `src/sim/miner/miner_system.rs:885-890`, `src/sim/movement/teleport_movement.rs:120-142`, `src/sim/movement/teleport_movement.rs:207-210`.  
**gamemd evidence:** `CHRONO_MINER_SYSTEM_OVERVIEW.md` documents self-teleport lock behavior, but `UnitClass__Mission_Harvest @ 0x0073E5E0` does not enter that branch for this close return coordinate.

Because Rust wrongly enters the teleport branch, it also applies the local harvester-specific zero-delay warp cleanup. This is downstream of F1, not an independent gamemd-close-path behavior.

**Player-visible difference:** the miner both relocates instantly and becomes available at the staging cell immediately, while retail is still driving/negotiating docking.

## Adjacent Findings

- The older manual-refinery trace's finding that Rust discarded the clicked refinery is stale for current source. Current `Command::MinerReturn` carries `target_refinery_id` and `world_commands` stores it as `reserved_refinery`.
- Exact command voice for manual return was not traced here.
- Exact dock-entry pivot/facing and dump-loop timing were not traced here.
- Current Rust uses `QueueingCell=4,1` for chrono return staging, while `refinery_can_dock_queue_cell` hardcodes `(rx+3, ry+1)` for CAN_DOCK. That coordinate distinction matters for a far-return trace, but the current scenario should not enter the far branch at all.

## Verdict Tally

PASS: 0 | FAIL: 4 | UNCHECKED: 6 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
