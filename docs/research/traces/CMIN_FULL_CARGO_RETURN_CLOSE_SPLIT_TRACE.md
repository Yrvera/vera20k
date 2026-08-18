# CMIN Full Cargo Return Close Split Trace

Date: 2026-05-23
Skill: `/trace-action`

Scenario: standard YR Allied Chrono Miner (`CMIN`) with full ore cargo is 3..50 cells from a same-owner stock Allied refinery (`GAREFN`) with one dock. Concrete computed case: `GAREFN` NW cell `(10,10)`, `CMIN` at `(40,40)`, full cargo, no other dock candidates, no active movement or teleport at trace start.

Scope guard: one mechanic only - full-cargo return branch, refinery selection, chrono close/far split, radio/drive versus teleport, and first visible return movement. Ghidra use was read-only decompile/search only.

## Player-Visible Verdict

Current Rust matches the active standard-YR close branch for this scenario. A full `CMIN` inside `ChronoHarvTooFarDistance=50` does not warp to the refinery queue cell. It selects the same-owner `GAREFN`, enters the refinery radio/contact path, and the first visible movement is ordinary driving toward the accepted dock cell `(NW+3,NW+1)`, not a chrono blink or teleport sound.

Older trace notes that cite a hardcoded 2-cell Rust threshold are stale for the current source: `src/sim/miner/miner_system.rs` now reads `config.too_far_threshold_chrono`, derived from parsed `ChronoHarvTooFarDistance`.

## Concrete Values

- Stock `rulesmd.ini`: `ChronoHarvTooFarDistance=50`, `CMIN Dock=NAREFN,GAREFN`, `CMIN Harvester=yes`, `CMIN Teleporter=yes`, `GAREFN DockUnload=yes`, `GAREFN NumberOfDocks=1`.
- Stock `artmd.ini`: `GAREFN QueueingCell=4,1`.
- Current Rust parsed config: `GeneralRules::chrono_harv_too_far_distance` default/INI value `50`; `MinerConfig::too_far_threshold_chrono = 50`.
- Concrete close-split distance: `(40,40)` to refinery object/NW `(10,10)` gives `dx=7680`, `dy=7680`, `dz=0` leptons. Squared distance is `117,964,800`; threshold is `50*256=12,800` leptons and squared threshold is `163,840,000`. Rust branch output is `distance_sq > threshold_sq == false`, so close radio path.
- Gamemd branch output: `UnitClass::Mission_Harvest` state 2 computes object-coordinate distance and compares it with `RulesClass+0xD7C * 0x100`; for this concrete distance the output is also close path.
- Accepted refinery dock cell for `GAREFN` NW `(10,10)`: gamemd `BuildingClass::Receive_Radio` case `0x0E` sends `NW+(3,1) = (13,11)`; Rust `refinery_can_dock_queue_cell(10,10)` also returns `(13,11)`.
- Far-return queue/staging cell, not used in this close scenario: art `QueueingCell=4,1` gives `(14,11)`.

## Pipeline

`full CMIN cargo` -> `Mission_Harvest return state` -> `Find_Docking_Bay / nearest GAREFN` -> `ChronoHarvTooFarDistance close split` -> `HELLO/radio contact` -> `Mission_Enter CAN_DOCK` -> `MOVE_TO_CELL (NW+3,NW+1)` -> visible drive toward dock cell.

## Stage Results

### Stage 1 - Stock Data And Rust Config

Gamemd active-YR data: `CMIN` is a live harvester teleporter with `Dock=NAREFN,GAREFN`; `GAREFN` is a live stock refinery with `DockUnload=yes` and one dock; `ChronoHarvTooFarDistance=50` is read from `RulesClass+0xD7C`.

Rust output: `GeneralRules` parses `ChronoHarvTooFarDistance`, and `MinerConfig::from_general_rules` copies it to `too_far_threshold_chrono`.

Verdict: PASS.

### Stage 2 - Refinery Selection

Gamemd output: `UnitClass::Mission_Harvest` state 2 calls the unit dock-list search; the single same-owner stock `GAREFN` is a valid dock candidate.

Rust output: `find_nearest_refinery` filters to friendly, refinery, dock-compatible, alive, complete structures and picks the only `GAREFN`.

Verdict: PASS for this one-refinery scenario.

### Stage 3 - Chrono Close/Far Split

Gamemd output: for a teleporter harvester, state 2 uses `ChronoHarvTooFarDistance * 0x100`; distance inside or equal to the threshold sends radio `2` to the refinery, while only distance greater than the threshold takes the far fallback.

Rust output: `chrono_return_exceeds_too_far_threshold` compares object-coordinate squared lepton distance with `(too_far_threshold_chrono*256)^2` and uses strict `>`. For `(40,40)` to `(10,10)`, Rust returns `false`, matching gamemd's close branch.

Verdict: PASS for the branch output. Exact gamemd rounded distance integer is not needed to decide this concrete branch because the value is far below `12,800`.

### Stage 4 - Close Radio / Contact Admission

Gamemd output: close branch sends refinery radio `2`; on accepted contact, `Mission_Harvest` switches to state 3, and state 3 queues `Mission_Enter`.

Rust output: `try_begin_chrono_close_return_radio` calls `hello_or_wait`; with one empty dock/contact slot it sets `MinerState::Dock`, `dock_phase=MissionEnter`, clears any movement target, and does not create teleport state.

Verdict: PASS for the visible state and no-teleport outcome.

### Stage 5 - First Visible Return Movement

Gamemd output: `Mission_Enter` / `BuildingClass::Receive_Radio` case `0x0E` sends `MOVE_TO_CELL(0x12)` to `NW+(3,1)`. For `GAREFN` at `(10,10)`, that cell is `(13,11)`.

Rust output: on the next dock phase tick, `phase_mission_enter` issues a direct move toward `refinery_can_dock_queue_cell(10,10) == (13,11)`.

Verdict: PASS for destination cell and drive-vs-teleport decision.

### Stage 6 - Teleport Effects And Sounds

Gamemd output: close branch does not use the far fallback Set_Destination-to-QueueingCell path and therefore does not produce inbound self-teleport visuals/sounds in this return split.

Rust output: `try_issue_chrono_far_return_teleport` is gated by the same over-threshold result, so this scenario does not call `spawn_warp_effects` or `issue_teleport_command`. Current regression coverage asserts no `teleport_state` and no `ChronoTeleport` sound for close return.

Verdict: PASS.

### Stage 7 - Exact Frame Timing

Gamemd output: decompile confirms state order, but this run did not use an instrumented gamemd replay/log to count exact frames from full-cargo state 2 to the first `0x12` move command.

Rust output: source/tests show close contact on the return tick and accepted-cell movement on the following dock tick.

Verdict: UNCHECKED.

### Stage 8 - Full Path Route And Pixel Movement

Gamemd output: the first accepted destination cell is verified, but the exact path nodes, subcell position, facing progression, and screen-pixel route from `(40,40)` to `(13,11)` were not computed from gamemd.

Rust output: pathing is delegated to direct movement/path systems after the accepted cell is assigned.

Verdict: UNCHECKED.

## Active-YR Confirmation

- `UnitClass::Mission_Harvest @ 0x0073E5E0` is the live harvester mission path; standard `CMIN` has `Harvester=yes`, `Teleporter=yes`, and a stock refinery dock list.
- The chrono close split reads `RulesClass+0xD7C`, matching stock `ChronoHarvTooFarDistance=50`.
- `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` is active for stock `GAREFN` because `DockUnload=yes`.
- The accepted move cell `NW+(3,1)` is active for stock `GAREFN`; `QueueingCell=4,1` belongs to far/wait staging, not the accepted close-radio movement cell.
- No TS-only fog/weed/Weeder branch is required for this standard YR scenario.

## Failures

None found in the scoped current Rust behavior.

## Not Implemented

None found in the scoped current Rust behavior.

## Adjacent Findings

- `CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md` has a stale Current Rust section saying Rust still uses a 2-cell chrono inbound threshold. The current source no longer matches that note.
- Queue contention, unload timing, and post-unload release are adjacent systems and were not traced here.

## References

- Rust: `src/sim/miner/miner_system.rs:38`, `:41`, `:631`, `:640`, `:643`, `:673`, `:677`, `:891`, `:903`, `:917`, `:946`, `:958`, `:985`, `:1063`, `:1181`
- Rust: `src/sim/miner/mod.rs:177`, `:217`, `:229`
- Rust: `src/rules/ruleset.rs:332`, `:1000`
- Rust: `src/sim/miner/miner_dock_sequence.rs:86`, `:104`, `:611`, `:659`, `:673`
- Rust tests: `src/sim/miner/miner_tests.rs:539`, `:1085`, `:1133`, `:1166`
- INI: `ini/rulesmd.ini:294`, `:7361`, `:7364`, `:7396`, `:11726`, `:11729`; `ini/artmd.ini:1773`
- Docs: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_SYSTEM_OVERVIEW.md`
- Docs: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md`
- Docs: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`
- Ghidra read-only: `UnitClass__Mission_Harvest @ 0x0073E5E0`
- Ghidra read-only: `BuildingClass__Receive_Radio @ 0x0043C2D0`
