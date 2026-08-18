# Miner FSM Full-Cargo Return and Refinery Reservation Trace

Scenario: a standard Yuri's Revenge Chrono Miner (`CMIN`) is harvesting ore, reaches full cargo, exits the harvest/search loop, selects an owned refinery, and begins the return/dock approach.

Scope boundaries: this trace stops at the beginning of return/dock approach. It does not trace unload credit math, exit drive, refinery-destroyed fallback, or multi-miner contention except where a single-miner stage already exposes the reservation protocol.

> **Repo-status supersession 2026-05-25:** References below to current Rust
> defining `CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2` are stale. Current Rust
> reads `ChronoHarvTooFarDistance` for the CMIN close/far return split. Do not
> use those line references as current implementation evidence.

## Evidence Used

- Live Ghidra read-only decompile in this run:
  - `UnitClass__Mission_Harvest @ 0073e5e0`
- Existing verified Ghidra reports:
  - `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
  - `MISSION_HARVEST_GHIDRA_REPORT.md`
  - `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md`
  - `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`
  - `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md`
  - `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`
  - `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- Rust source:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/mod.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_spawn.rs`
- INI data:
  - `ini/rulesmd.ini`
  - `ini/artmd.ini`

## Pipeline

`CMIN harvest tick` -> `full-storage check` -> `Mission_Harvest state 2 / Rust ReturnToRefinery` -> `find owned dockable refinery` -> `HELLO/contact or Rust reserved_refinery` -> `Mission_Enter/CAN_DOCK or Rust DockReservations` -> `queue/pad movement begins`.

## Stage Results

### Stage 1 - CMIN and refinery data

Our path:
- `world_spawn.rs:395-398` attaches `Miner::new(kind, config, storage)` from parsed object storage.
- `mod.rs:298-313` uses `Storage=` over the kind default.
- `rulesmd.ini:7351-7398` gives `CMIN`: `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=20`, `Teleporter=yes`.
- `rulesmd.ini:11722-11729` gives `GAREFN`: `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`.
- `artmd.ini:1763-1773` gives `GAREFN`: `Foundation=4x3`, `QueueingCell=4,1`.

gamemd:
- `UnitClass__Mission_Harvest @ 0073e5e0` reads `Type+0xe0e` (`Harvester`), `Type+0xcd4` (`Teleporter`), `Type+0x3f8` dock-list count, and `Type+0x800` storage capacity.
- Existing reports map `Storage=20`, `Dock=NAREFN,GAREFN`, and `Teleporter=yes` to those fields.

Verdict: PASS for these data values. The concrete values match: CMIN capacity is 20 bales, CMIN is a teleporter harvester, and GAREFN is a one-dock refinery.

### Stage 2 - Full-cargo trigger

Our path:
- `miner_system.rs:491-510` computes empty capacity, extracts up to that many bales, extends `cargo`, then calls `is_full()`.
- `mod.rs:337-340` defines full as `cargo.len() >= capacity_bales`.
- With `Storage=20`, a 19-bale CMIN receiving one ore bale becomes `20 >= 20`.

gamemd:
- In `UnitClass__Mission_Harvest @ 0073e5e0`, state 1 calls `UnitClass__Harvest_Ore_Tick`, then on failure checks `Get_Storage_Percentage() == 1.0` and writes state 2.
- State 0 also has a full check using `Get_Storage_Percentage() >= 1.0`.
- With `Storage=20`, 20 carried bales gives `20 / 20 = 1.0`.

Verdict: PASS for the full/not-full decision in the exact 20-bale case.

### Stage 3 - Full transition side effects

Our path:
- `miner_system.rs:504-510` saves archive via short scan, then calls `begin_return`.
- `miner_system.rs:859-912` selects a refinery and sets `MinerState::ReturnToRefinery`.

gamemd:
- `UnitClass__Mission_Harvest @ 0073e5e0` writes harvest substate 2 after full, runs a short scan using `RulesClass+0x1778` (`TiberiumShortScan=6`) to update the ghost/archive cell, then returns 1.

Verdict: UNCHECKED. The high-level side effect exists in both paths, but this run did not compute identical archive cells for a concrete map layout.

### Stage 4 - Refinery selection

Our path:
- `miner_system.rs:866-873` calls `find_nearest_refinery`, then stores `reserved_refinery = Some(rsid)`.
- `miner_system.rs:997-1038` iterates structures, requires friendly owner, refinery type, `Dock=` compatibility, alive health, not building-up, and chooses minimum squared distance to the Rust dock cell.

gamemd:
- `UnitClass__Mission_Harvest @ 0073e5e0` state 2 calls vtable `+0x528`, `FootClass::Find_Docking_Bay`, over the unit type's dock list.
- Existing reports say it selects the closest valid dock building using 3D lepton distance and the unit's dock list.

Verdict: UNCHECKED. With exactly one owned compatible refinery the selected refinery matches, but this trace did not compute a multi-refinery tie/order case with both engines.

### Stage 5 - Chrono return branch threshold and first action

Our path:
- `miner_system.rs:36-45` defines `CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2`.
- `miner_system.rs:874-906` compares cell distance to that 2-cell threshold and, when farther, immediately targets the refinery pad with `issue_teleport_command`.

gamemd:
- `UnitClass__Mission_Harvest @ 0073e5e0` state 2 reads `RulesClass+0xD7C` (`ChronoHarvTooFarDistance=50`) and compares `distance <= 50 * 0x100 = 12800` leptons.
- If that comparison passes, the active YR code sends `Transmit_Radio(2, refinery)` (`HELLO`) and, on return value 1, writes harvest substate 3.
- The fallback passable-cell destination path is after the failed/too-far branch, not after a 2-cell threshold.

Verdict: FAIL. For an ordinary ore-field distance such as 10 cells from the refinery center, our output is "warp to pad/ReturnToRefinery"; gamemd output is "send HELLO to refinery and enter Mission_Harvest substate 3."

### Stage 6 - HELLO/contact admission

Our path:
- `miner_system.rs:873` records `reserved_refinery` before any building-side contact equivalent.
- `miner_dock_sequence.rs:416-423` does not claim `DockReservations` until later, after top-level state has become `Dock`.
- There is no RadioClass contact roster or HELLO return code in the Rust miner path.

gamemd:
- `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` verifies `HELLO(2)` is handled by `RadioClass::Receive_Radio`.
- For `NumberOfDocks=1`, `HELLO` accepts only if a contact slot is free or already contains the sender; accepted output is `Contacts[i] = sender` and return value 1.
- `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md` verifies radio `0x10` reserve is not sent by the active YR harvester-refinery chain.

Verdict: NOT-IMPLEMENTED. The Rust path has a local `reserved_refinery` field and later `DockReservations`, but no active-YR HELLO/contact admission stage.

### Stage 7 - Queue/dock target cell

Our path:
- `miner_system.rs:1028-1029` passes parsed `queueing_cell` into `refinery_dock_cell`.
- `miner_dock_sequence.rs:65-76` returns `rx + qx, ry + qy` when `QueueingCell=` exists.
- For GAREFN at anchor `(rx, ry)`, parsed `QueueingCell=4,1` gives `(rx+4, ry+1)`.

gamemd:
- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` verifies active `BuildingClass::Receive_Radio @ 0043c2d0`, case `0x0E`, computes queue cell as building anchor `(x+3, y+1)`.
- The report explicitly verifies this path does not read stored `QueueingCell=` fields.

Verdict: FAIL. For GAREFN at `(10,10)`, our queue target is `(14,11)` while gamemd's CAN_DOCK `MOVE_TO_CELL` target is `(13,11)`.

### Stage 8 - Dock claim timing

Our path:
- `miner_dock_sequence.rs:416-423` calls `DockReservations::try_reserve`; if granted, it immediately inserts the miner into `occupied`, issues a direct move to the pad, and switches to `Linked`.
- `miner_dock.rs:30-44` models the granted dock as `occupied[refinery_sid] = miner_sid`.

gamemd:
- HELLO only writes `Contacts[]`.
- `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` verifies `BuildingClass+0x2E4` is the on-pad unit pointer and is not written by HELLO.
- `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` verifies CAN_DOCK sends movement/timing messages and does not write `+0x2E4`; the on-pad link is set only when the unit physically arrives.

Verdict: FAIL. Our first successful local reservation is already an occupied dock claim before pad arrival; gamemd separates contact/queue admission from the physical on-pad claim.

### Stage 9 - Start of return/dock approach

Our path:
- Farther than 2 cells, `begin_return` spawns warp effects and `issue_teleport_command(..., is_harvester=true)` toward the pad (`miner_system.rs:884-903`).
- Subsequent `handle_return` waits for teleport state, then transitions to `Dock` if adjacent to the Rust dock cell (`miner_system.rs:588-656`).

gamemd:
- For distance `<= 50*256`, state 2 sends HELLO, state 3 sets mission 7 (`Mission_Enter`), then `FootClass::Mission_Enter`/`TechnoClass::Set_Destination` and `BuildingClass::Receive_Radio` case `0x0E` drive the queue/dock approach.
- Whether a chrono warp occurs later is controlled by the destination cell and locomotor path, but the first state-2 output for this branch is not "warp to pad"; it is HELLO and mission handoff.

Verdict: FAIL for the first return/dock approach output on the normal within-50-cell YR branch.

### Stage 10 - Render/audio at this boundary

Our path:
- `spawn_warp_effects` emits departure-only warp visuals and chrono in/out sounds (`miner_system.rs:926-990`).

gamemd:
- The active state-2 within-threshold branch does not spawn chrono effects at that point; chrono visuals belong to later locomotor movement if a teleport destination is issued.

Verdict: UNCHECKED. This trace did not compute a full later Mission_Enter locomotor path and therefore cannot declare exact visual/audio equality.

## Failures

1. Stage 5: Return threshold/action mismatch. The Rust miner uses a hardcoded 2-cell chrono return gate and may warp to pad immediately; gamemd uses `ChronoHarvTooFarDistance=50` as the state-2 HELLO branch threshold.
2. Stage 6: The active YR HELLO/contact roster is absent. Rust has `reserved_refinery` and later `DockReservations`, but not the synchronous `Transmit_Radio(2)` acceptance output.
3. Stage 7: Queue cell differs. Rust uses `QueueingCell=4,1`; gamemd active CAN_DOCK uses hardcoded `+3,+1`.
4. Stage 8: Dock claim timing differs. Rust marks the dock occupied at `DockReservations::try_reserve`; gamemd does not set the on-pad unit link until physical pad arrival.
5. Stage 9: The first return/dock approach output differs for normal within-50-cell chrono miner returns.

## Not Implemented

- Active-YR RadioClass contact/HELLO admission for refinery return is not represented in the Rust miner FSM.

## Adjacent Findings

- Radio `0x10` reserve is receiver-ready but has no active standard-YR harvester sender; it should not be implemented as the normal refinery reservation path.
- Multi-miner contention, refinery unavailable mid-cycle, and manual return were intentionally not traced here.

## Verdict Tally

PASS: 2 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
