# Two Chrono Miners Same Refinery Full Cargo Queue Takeover Trace

Date: 2026-05-22
Skill: `/trace-action`
Scenario: Standard YR skirmish, two same-owner full `CMIN` Chrono Miners returning to one same-owner stock refinery (`GAREFN` or `NAREFN`) with `NumberOfDocks=1`.
Scope: return trigger -> first miner dock/unload/stock state-4 exit -> second miner wait/retry -> second miner takeover -> player-visible result.

No Rust, INI, or existing research docs were edited. This trace document is the only saved artifact.

> **Repo-status supersession 2026-05-25:** The finding, stage row, tally, and
> follow-up about a hardcoded 2-cell chrono inbound threshold are stale against
> current Rust. Current Rust reads `ChronoHarvTooFarDistance` and uses it for
> the close/far split. Preserve the remaining unload timing and end-to-end
> takeover findings.

## Player-Visible Findings

1. STALE/FIXED vs current Rust: this trace originally reported a hardcoded `> 2` cell inbound chrono-warp threshold. Current Rust now uses parsed `ChronoHarvTooFarDistance` for the state-2 close-radio vs far-fallback split. Keep regression coverage for the 3..50 cell band, but do not treat the old threshold note as a current mismatch.
2. FAIL: the old "hold for full SpecialAnim" bug is fixed, but the current unload completion still appears to add one extra dump-gate hold. Rust reaches `next_slot == None` only after the post-last-slot dump gate has already elapsed, then seeds another 15-tick `DepositCooldown` before releasing the dock. Gamemd's state 3 transitions to state 4 on that empty-slot gate, then state 4 can complete on the following mission tick if `building+0x57C == 0`. Evidence: `src/sim/miner/miner_dock_sequence.rs:840`, `src/sim/miner/miner_dock_sequence.rs:847`, `src/sim/miner/miner_dock_sequence.rs:856`, `src/sim/miner/miner_dock_sequence.rs:879`, and Ghidra spot-check of `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
3. FIXED vs prior trace: the second far-returning Chrono Miner no longer warps directly to the occupied pad/footprint in current Rust. It stages at art `QueueingCell=4,1`, `(14,11)` for a refinery at `(10,10)`, not the accepted pad `(13,11)`. Evidence: `src/sim/miner/miner_system.rs:864`, `src/sim/miner/miner_system.rs:1042`, and test `chrono_miner_teleports_to_refinery_on_return`.
4. FIXED vs prior trace: normal stock state-4 exit no longer issues `Force_Track(0x47)`, no explicit queue-cell exit move, and no `bypass_grid` overlap drive through the waiting miner. Evidence: `src/sim/miner/miner_dock_sequence.rs:888`, tests `stock_departing_does_not_start_force_track_0x47`, `stock_departing_does_not_start_explicit_exit_move`, and `departing_handoff_ignores_blocked_queue_cell`.
5. UNCHECKED: exact full end-to-end takeover frame where miner B starts moving from `(14,11)` to `(13,11)` after miner A's stock state-4 release is not proven equal. Current tests pin the pieces, but no single test computes two full `CMIN` cargos through first unload, release, second move, and second unload with occupancy checked at every frame.

## Data

| Item | Stock value | Evidence | Status |
|---|---:|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | PASS |
| `[CMIN] Harvester` | `yes` | `ini/rulesmd.ini:7364` | PASS |
| `[CMIN] Speed` | `4` | `ini/rulesmd.ini:7369` | PASS |
| `[CMIN] Storage` | `20` | `ini/rulesmd.ini:7374` | PASS |
| `[CMIN] UnloadingClass` | `CMON` | `ini/rulesmd.ini:7384` | PASS |
| `[CMIN] Teleporter` | `yes` | `ini/rulesmd.ini:7396` | PASS |
| `[GAREFN] DockUnload/Refinery/NumberOfDocks` | `yes/yes/1` | `ini/rulesmd.ini:11726-11729` | PASS |
| `[NAREFN] DockUnload/Refinery/NumberOfDocks` | `yes/yes/1` | `ini/rulesmd.ini:12519-12521` | PASS |
| `[GAREFN]/[NAREFN] Foundation` | `4x3` | `ini/artmd.ini:1766`, `1709` | PASS |
| `[GAREFN]/[NAREFN] QueueingCell` | `4,1` | `ini/artmd.ini:1773`, `1716` | PASS |

## Pipeline

`CMIN full cargo` -> `Mission_Harvest return` -> `close HELLO or far QueueingCell fallback` -> `Mission_Enter CAN_DOCK retry` -> `0x12 accepted cell (rx+3, ry+1)` -> `0x18/0x16 entered/pivot` -> `0x15 queues sender mission 0x10` -> `Mission_Deploy_Building zero-link state 3 unload` -> `empty-slot gate sets state 4` -> `state 4 clears dock-active/contact and queues Harvest` -> `next miner retries/takes over`.

## Stage Table

| Stage | Gamemd output | Current Rust output | Verdict |
|---|---|---|---|
| 1. Scenario data | `CMIN` is `Harvester=yes`, `Teleporter=yes`, `Storage=20`; stock refineries have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `QueueingCell=4,1`. | Parser-facing data matches these INI keys in the inspected rule files. | PASS for source data. Runtime parser equality not separately re-run. |
| 2. Return trigger and refinery selection | `UnitClass::Mission_Harvest @ 0x0073E5E0` uses the unit dock list and stock refinery flags. | `begin_return` calls `find_nearest_refinery`, writes `reserved_refinery`, then enters return/dock handling. | UNCHECKED. Same-owner candidate ordering was not computed against gamemd for a concrete map. |
| 3. Chrono close/far split | CMIN close path uses `ChronoHarvTooFarDistance=50`; far fallback seeds `QueueingCell` and passable-cell search. | Current Rust uses parsed `ChronoHarvTooFarDistance` for the close/far split. | PASS for source-level threshold selection. Full end-to-end timing for two loaded `CMIN`s was not recomputed in this trace. |
| 4. Far-return staging destination | For far fallback, seed is refinery anchor `+(4,1)`, then `Find_Nearby_Passable_Cell`. For `(10,10)`, seed is `(14,11)`. | `chrono_return_staging_cell_for_sid` uses `refinery_queue_cell`; test pins target `(14,11)`. | PASS for unblocked stock `(10,10)` staging. |
| 5. HELLO/contact admission | `HELLO(0x02)` populates `Contacts[]`; stock `NumberOfDocks=1` allows one contact. Incoming full HELLO rejects without evicting current contact. | `RefineryDockContacts::hello_or_wait` accepts one contact and pushes later miners to FIFO `waiting_retry_queue` without evicting current contact. | PASS for two-miner one-contact behavior. |
| 6. Busy CAN_DOCK retry | Receiver `0x0E` can return `1` without `0x18/0x16`; no free contact slot alone is not a hard `10` in the standard DockUnload branch. | `phase_mission_enter` keeps the waiter in `MissionEnter`, sets `dock_queued`, and does not mark `contact_entered` while pad/contact is busy. | PASS for visible "wait/retry, do not enter" behavior. Internal radio shape differs. |
| 7. Accepted cell vs queue cell | Accepted `0x12` target is hardcoded anchor `+(3,1)`, `(13,11)` for `(10,10)`. `QueueingCell=4,1` is not read by accepted `0x0E`. | `refinery_can_dock_queue_cell` returns `(rx+3, ry+1)`; `refinery_queue_cell` separately returns `(rx+4, ry+1)`. | PASS. |
| 8. Pad/entered link | Only when the unit is already at accepted cell does `0x0E` send `0x18/0x16`; pad arrival later sends `0x15`. | `phase_mission_enter` waits for accepted-cell already-there, then marks `contact_entered`; `phase_linked` marks `on_pad`. | PASS for tested cell/state outputs. Exact frame-perfect pivot is outside this queue trace. |
| 9. First miner unload grain and credits | `Mission_Deploy_Building` state 3 drains one whole StorageClass slot per `HarvesterDumpRate * 900 = 14.4` frame gate. | `phase_unloading` drains all bales of one resource type per gate and emits one `BaleDepositEvent`. | PASS, inherited from `CHRONO_MINER_ORE_DUMP_DEPOSIT_TRACE.md` and current code. |
| 10. Cargo-empty state-4 timing | After the last slot drain, the next 14.4-frame gate finds no slot, sets state 4, and state 4 can exit on the next mission tick if `building+0x57C == 0`. | After `next_slot == None`, Rust sets `deposit_cooldown_ticks = ceil(14.4) = 15`, then needs a pass-through tick to `Departing`, then one more tick to release. | FAIL. The Rust empty-slot branch appears to double-hold by one dump interval before release. |
| 11. Stock exit shape | Zero-link state 4 clears `+0x6D1`, sets mission Harvest `0x0A`, optionally sends BREAK `3`, queues mission; no normal `ReleaseDockedHarvester` or `Force_Track(0x47)`. | `phase_departing` releases pad/contact, clears override/targets, resets dock phase, sets `SearchOre`, and does not issue force track or explicit exit move. | PASS for no Force_Track/no explicit queue-cell move. Exact mission-label equivalence is UNCHECKED. |
| 12. Queue-cell overlap / bypass | Gamemd stock exit does not use `Force_Track(0x47)` for normal zero-link exit; queue handoff is protocol-driven, not a bypass drive through the waiter. | No stock exit move is issued, and no miner-side `bypass_grid=true` assignment remains in the dock sequence. | PASS for the old overlap-by-exit-drive failure being fixed. Full A/B same-tick occupancy after release is UNCHECKED. |
| 13. Second miner takeover | Once contact/pad state clears, second miner retries `CAN_DOCK`, moves from wait cell `(14,11)` to accepted/pad cell `(13,11)`, and begins its own unload. | Tests show a waiter remains deferred while contact/pad are occupied and enters after contact and pad are released. No full two-CMIN cargo trace computes every tick. | UNCHECKED for exact full scenario timing; partial PASS for state transition pins. |

## Current Rust Evidence

- Far chrono return now stages at `QueueingCell`, not the pad: `src/sim/miner/miner_system.rs:864-891`, `src/sim/miner/miner_system.rs:1030-1061`; test `chrono_miner_teleports_to_refinery_on_return` expects `(14,11)`.
- HELLO/contact/wait queue uses one contact and FIFO retry order: `src/sim/miner/miner_dock.rs:42-78`; tests `dock_queuing_one_at_a_time`, `occupied_can_dock_defers_without_clearing_waiting_miner_target`.
- Accepted cell is separated from wait cell: `src/sim/miner/miner_dock_sequence.rs:86-105`, `src/sim/miner/miner_dock_sequence.rs:335-340`; test `hello_before_mission_enter_then_can_dock_move` expects `(13,11)`.
- Stock state-4 exit does not drive through the queue cell: `src/sim/miner/miner_dock_sequence.rs:879-923`; tests `stock_departing_hands_directly_to_search_without_exit_move`, `stock_departing_does_not_start_force_track_0x47`, `departing_handoff_ignores_blocked_queue_cell`.
- Focused verification run: `cargo test -q -p vera20k sim::miner::miner_tests` passed, 89 tests passed.

## Binary/Doc Reconciliation

`RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` is YELLOW in `AUDIT_LOG.md:182`, so I did not rely on its stale wording for the 0x15 handoff or `+0x418` meaning. Targeted Ghidra decompile of `BuildingClass::Receive_Radio @ 0x0043C2D0` confirmed:

- case `0x0E` computes stock DockUnload `0x12` payload as building NW `+(3,1)`;
- `0x18`/`0x16` are sent only after `0x12` returns already-there `0x14`;
- case `0x15` queues mission `0x10` on the sender miner, not the refinery;
- no reciprocal `+0x2E4` write appears in these cases.

`MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` is RED in `AUDIT_LOG.md:183`, so this trace uses the newer branch/writer reports plus a targeted decompile of `UnitClass::Mission_Deploy_Building @ 0x0073D630`. The spot-check confirmed:

- `PathType::Has_Valid_Steps` true proceeds into the harvester timer/state dispatch;
- stock standard unload normally runs with `unit+0x2E4 == 0`;
- state 3 drains cargo and sets state 4 on `FindFirstNonEmptySlot == -1`;
- state 4 waits on `building+0x57C`, clears `+0x6D1`, sets mission Harvest `0x0A`, optionally sends radio `3`, and queues the next mission;
- `ReleaseDockedHarvester` is only on the nonzero `+0x2E4` branch, not normal cargo-empty exit.

## Verdict Tally

PASS: 7
FAIL: 1 current, 1 stale/fixed vs current Rust
UNCHECKED: 4
NOT-IMPLEMENTED: 0

## Highest-Leverage Follow-Ups

1. Keep/extend regression coverage for the parsed `ChronoHarvTooFarDistance` close/far split, especially the 3..50 cell band that the stale trace used to flag. No current threshold replacement is needed.
2. Rework unload completion so `next_slot == None` is the empty-slot gate itself, not the start of another full dump-gate hold. Add a test that starts immediately after the last slot drain and counts ticks to contact release.
3. Add a single end-to-end two-`CMIN` test: both full, one refinery, miner A lower stable id, miner B waiting at `(14,11)`, assert no pad overlap, exact release tick, and miner B's first accepted-cell move/unload start.
4. If exact parity is required before implementation, run a live gamemd replay/log trace for the same `(10,10)` refinery and two known miner coordinates to pin the frame where B leaves the queue cell.

## Sources

- `docs/research/miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md`
- `docs/research/miner/traces/CHRONO_MINER_ORE_DUMP_DEPOSIT_TRACE.md`
- `docs/research/miner/MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`
- `docs/research/miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`
- `docs/research/BUILDING_RECEIVE_RADIO_REFINERY_0X0E_NON_ACCEPTED_PATHS_GHIDRA_REPORT.md`
- Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra decompile: `FUN_0065ADF0 @ 0x0065ADF0`
- `ini/rulesmd.ini`, `ini/artmd.ini`
- Rust: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_tests.rs`
