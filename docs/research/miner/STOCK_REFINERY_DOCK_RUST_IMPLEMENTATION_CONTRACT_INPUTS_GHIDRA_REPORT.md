# Stock Refinery Dock Rust Implementation Contract Inputs - Ghidra Research Report

**Date:** 2026-05-24  
**Target:** stock YR `CMIN/HARV -> GAREFN/NAREFN` docking and unload handoff inputs for a future Rust implementation contract.  
**Investigation Mode:** coverage-map from verified current docs plus focused Ghidra spot-checks.  
**Claimed Scope:** Rust-facing states, gates, tests, affected surfaces, and negative facts for the stock refinery dock admission/unload split.  
**Non-Scope:** Rust implementation, final implementation-contract doc, slave miners, service depots, aircraft docks, modded multi-dock buildings, save/load reconstruction, and exact first-rendered-frame capture.  
**Confidence:** High for the static binary states/gates listed below; Medium for exact first winning `0x15` source in every retail replay frame.  
**Active in YR:** Yes. Stock `rulesmd.ini` has `[CMIN] Dock=NAREFN,GAREFN`, `[HARV] Dock=NAREFN,GAREFN`, `[GAREFN]/[NAREFN] DockUnload=yes`; `artmd.ini` has `[GAREFN]/[NAREFN] QueueingCell=4,1`.

## 0. Working Notes

**Target question:** What precise verified facts must a Rust implementation contract consume for stock CMIN/HARV refinery docking, and which current Rust files/tests are likely affected?  
**Non-goals:** Do not write Rust; do not create the final implementation-contract doc; do not expand into slave miner/service depot/aircraft dock behavior; do not collapse unresolved first-frame race questions into false certainty.  
**Evidence needed to mark COMPLETE:** current canonical synthesis plus the six named focused reports; at least two load-bearing Ghidra read-only spot-checks; focused Rust scan with likely files/functions and existing tests; final report written to this path.  
**Stop conditions:** stop after verified contract inputs are listed, Active-in-YR status is stated, Rust surfaces/tests are identified, and remaining uncertainty is explicit.

## 1. Source Inputs Used

Current canonical doc:

- `miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`

Latest focused reports:

- `FOOTCLASS_MISSION_ENTER_0X0E_REPEAT_TIMING_GHIDRA_REPORT.md`
- `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
- `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`
- `BUILDING_RECEIVE_RADIO_0E_GETDOCKCOORD_SIDE_CHECK_GHIDRA_REPORT.md`
- `DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`
- `REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`

Additional read-only spot checks in this report:

- `BuildingClass__Receive_Radio @ 0x0043C2D0`
- `UnitClass__Receive_Radio @ 0x00737430`
- `FootClass__Mission_Enter @ 0x004D9290`
- `UnitClass__PerCellProcess @ 0x00739EC0`

## 2. Load-Bearing Ghidra Spot Checks

### 2.1 Building `0x0E` sends accepted cell NW+(3,1), then gates `0x18/0x16` on `0x12 == 0x14`

**Verified binary fact:** `BuildingClass__Receive_Radio(0x0E)` in the `DockUnload || Weeder` branch computes `building.Get_Cell_Packed() + (3,1)`, converts that to a `CellClass*`, sends `0x12`, and returns immediately unless the reply is exactly `0x14`. Only the already-there reply sends `0x18` then `0x16`.  
**Evidence:** Ghidra read-only decompile `0x0043C2D0`; decompiled block has `CONCAT22(psVar5[1] + 1,*psVar5 + 3)`, `MapClass__Get_CellClass`, `vtable+0x27c(0x12, ...)`, `if (iVar10 != 0x14) return 1`, then `vtable+0x278(0x18, ...)` and `vtable+0x278(0x16, ...)`.  
**Active in YR:** Yes, for stock GAREFN/NAREFN through `DockUnload=yes`.

### 2.2 Building early `GetDockCoord` is a side-check, not the accepted target

**Verified binary fact:** the earlier `GetDockCoord` touch in building `0x0E` is before the accepted payload. It masks the requester as Foot-derived, calls building vtable `+0xA8`, converts that coordinate to a `CellClass*`, compares with requester `+0x5A4`, and only affects a local allowance byte for later non-ROGER `0x13` continuation.  
**Evidence:** Ghidra read-only decompile `0x0043C2D0` shows this side-check in the same function before the later `+3,+1` payload; focused report gives assembly ranges `0x0043C8E2..0x0043C93A` and consumer `0x0043CA07..0x0043CA0D`.  
**Active in YR:** Yes, conditional on contact membership and `DockUnload=yes`.

### 2.3 Unit `0x16` first call can be timer-only; later call can send `0x15` without `GetDockCoord`

**Verified binary fact:** `UnitClass__Receive_Radio(0x16)` first calls `FootClass__Receive_Radio`, then if `+0x6AF == 0` and `RateTimer::Current(+0x388) != 0x4000`, calls locomotor vtable `+0x4C(0x4000)` and returns `1`. If already synchronized, it checks `Is_Moving()==false`, `FootClass__GetDestination`, contact flag `+0x418`, destination `WhatAmI()==6`, and receiving unit mission `7`, then sends `0x15`.  
**Evidence:** Ghidra read-only decompile `0x00737430`, case `0x16`, shows the timer branch, immediate return, later locomotor `+0x10` moving check, destination/building/mission gates, and `vtable+0x278(0x15, piVar5)`.  
**Active in YR:** Yes, for stock refinery `0x18/0x16` handoff.

### 2.4 Mission Enter retry is timer-gated, not next-tick polling

**Verified binary fact:** `FootClass__Mission_Enter` sends one `0x0E` per dispatch and exits through `MissionClass__GetMissionTimerEntry`, `Math__ftol`, and `Random__RandomRanged(0,2)`. Stock `[Enter] Rate=.016` makes `14..16` frames.  
**Evidence:** Ghidra read-only decompile `0x004D9290`; focused report ties dispatch storage to `MissionClass__Mission_Dispatch @ 0x005B3060`; `rulesmd.ini:[Enter] Rate=.016`.  
**Active in YR:** Yes, stock miner Enter mission.

### 2.5 PerCellProcess `0x15` remains source-separate

**Verified binary fact:** `UnitClass__PerCellProcess @ 0x00739EC0` has a destination-building `GetDockCoord` equality branch that sends `0x15`, and a separate later contact-flag adjacent-building `0x15` branch. It is not the mission-7 dispatch handler.  
**Evidence:** Ghidra read-only decompile `0x00739EC0` shows the current-cell vs destination building `vtable+0xA8` comparison and `vtable+0x274(0x15)`, plus the later `field_0x418` branch sending `0x15` to `FootClass__GetDestination`.  
**Active in YR:** Yes, conditional on physical cell-entry/contact gates.

## 3. Required Rust-Facing State Model

| Required state / split | Verified source | Rust-facing purpose | Active in YR |
|---|---|---|---|
| `HELLO(0x02)`/contact admission before Enter | synthesis; building reports | separate refinery contact list from movement/unload start | Yes |
| Mission `7` / `Enter` retry timer | `0x004D9290`, `0x005B3060`; Enter report | `0x0E` must not poll every tick; stock retry `14..16` frames | Yes |
| Accepted `0x12` target = NW+(3,1) | `0x0043C2D0`; synthesis | movement assignment target; not `GetDockCoord` | Yes |
| Stopped at accepted cell with refinery destination still live | DriveLocomotor report | movement can be idle while dock destination/contact logic continues | Yes |
| Already-there `0x12 == 0x14` handoff | `0x0043C2D0`; Enter report | only this path sends `0x18/0x16` | Yes |
| Contact-entered flag `+0x418` from `0x18` | `0x006F4AB0`; 0x16/Drive reports | separate from contact admission and pad occupancy | Yes |
| First unsynced `0x16` | `0x00737430`; 0x16 report | may only start facing timer and return | Yes |
| Later/already-synced `0x16 -> 0x15` | `0x00737430`; 0x16 report | can start unload handoff from stopped accepted cell; no GetDockCoord equality | Yes |
| Per-cell `GetDockCoord` `0x15` branch | `0x00739EC0`; PerCellProcess reports | separate physical cell-entry source | Yes |
| Contact-flag adjacent-building `0x15` branch | `0x00739EC0`; Drive report | second PerCellProcess source, source-aware and timing-sensitive | Conditional |
| Mission `0x10` / `Mission_Deploy_Building` unload FSM | synthesis; older deploy report | deposit/drain must stay after `0x15`, not admission | Yes |
| Zero-link state-4 release | synthesis; two-miner reports | clear visual/contact and queue Harvest/Search; no waiter promotion | Yes |

## 4. Required Gates And Ordering

| Gate / ordering | Required contract input |
|---|---|
| Stock liveness | Require stock CMIN/HARV `Dock=NAREFN,GAREFN`; GAREFN/NAREFN `DockUnload=yes`; `QueueingCell=4,1` is not accepted cell. |
| Coordinate split | Keep accepted NW+(3,1), stock `GetDockCoord` NW+(2,1), and `QueueingCell` NW+(4,1) as named, separate reference points. |
| `0x12 == 1` | Assign/continue movement only; no `0x18`, no `0x16`, no unload start in that pass. |
| `0x12 == 0x14` | Send `0x18`, then `0x16`, synchronously inside building `0x0E`; Mission Enter still returns stock delay. |
| First `0x16` | If facing timer not already `0x4000`, set turn timer through locomotor and return `1`; do not infer `0x15` from return `1`. |
| Later `0x16` | Requires stopped, destination flag/contact, destination building, receiving unit mission `7`; sends `0x15`; no `GetDockCoord` compare. |
| PerCellProcess | Runs after mission dispatch/locomotor callback order described in the caller tick-order report; its `GetDockCoord` branch fails at accepted NW+(3,1). |
| Completion | Normal stock unload completion is Mission_Deploy_Building state 4 with zero reciprocal dock link; optional `BREAK(3)` if valid contact; no refinery-side waiter promotion. |

## 5. Focused Rust Scan

No Rust files were edited.

Likely affected implementation surfaces:

- `src/sim/miner/mod.rs:86` `RefineryDockPhase`: already has `Approach`, `MissionEnter`, `AwaitingAcceptedCell`, `Linked`, `Pivoting`, `Unloading`, `DepositCooldown`, `Departing`.
- `src/sim/miner/miner_dock_sequence.rs:86` `refinery_queue_cell`: uses `QueueingCell`.
- `src/sim/miner/miner_dock_sequence.rs:104` `refinery_can_dock_queue_cell`: returns accepted `NW+(3,1)`.
- `src/sim/miner/miner_dock_sequence.rs:116` `refinery_pad_cell`: returns/derives pad/GetDockCoord-like `NW+(2,1)`.
- `src/sim/miner/miner_dock_sequence.rs:613` `phase_mission_enter`: current already-at-accepted branch marks contact-entered and enters `Linked`.
- `src/sim/miner/miner_dock_sequence.rs:680` `phase_awaiting_accepted_cell`: returns to `MissionEnter` after movement completion, but current scan found no explicit `14..16` Enter timer gate here.
- `src/sim/miner/miner_dock_sequence.rs:700` `phase_linked`: snapshots miner to pad, marks `on_pad`, emits deploy sound, starts pivot; this is the main compression risk because binary has source-aware `0x16 -> 0x15` and PerCellProcess `0x15` sources.
- `src/sim/miner/miner_dock_sequence.rs:793` `phase_unloading`: drains slots and awards credits.
- `src/sim/miner/miner_dock_sequence.rs:900` `phase_departing`: implements zero-link state-4 cleanup and releases contact/pad.
- `src/sim/miner/miner_dock.rs:31` `RefineryDockContacts`: contacts, waiting queue, contact-entered, and on-pad are already separate.
- `src/sim/miner/miner_dock.rs:124` `release_contact`: clears contact without promoting waiters.
- `src/sim/miner/miner_system.rs:89` `tick_miners`: deterministic stable-id snapshot order is relevant to two-miner timing.
- `src/sim/movement/movement_tick.rs` and `src/sim/components.rs`: movement completion currently clears `movement_target`; future exactness needs a logical refinery destination/contact state independent of movement target.

Existing tests to keep/extend:

- `release_contact_does_not_promote_waiter`
- `chrono_miner_teleports_to_refinery_on_return`
- `return_close_enough_to_refinery_enters_dock`
- `chrono_return_close_enough_enters_radio_dock_without_can_dock_move`
- `war_miner_close_return_uses_accepted_cell_not_queueingcell`
- `cmin_refused_close_return_stages_at_queueingcell_then_can_dock_uses_accepted_cell`
- `refinery_pad_cell_matches_stock_garefn_hardcoded_offset`
- `accepted_cell_arrival_rechecks_can_dock_before_entered_flag`
- `waiter_moves_from_queueingcell_to_accepted_cell_before_entered`
- `occupied_can_dock_defers_without_clearing_waiting_miner_target`
- `queued_miner_enters_after_contact_and_pad_are_released`
- `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`
- `two_miners_waiter_before_releaser_not_retroactively_promoted`
- `accepted_cell_arrival_sets_contact_entered_then_0x15_starts_unload_fsm`
- `departing_handoff_releases_dock_and_returns_to_search`
- `dock_first_slot_drain_waits_one_unload_interval`
- `empty_unload_gate_releases_dock_on_next_stock_state4_handoff`

Proposed missing/renamed focused tests for the implementation contract:

- `mission_enter_accepted_cell_recheck_waits_enter_rate_delay`
- `mission_enter_already_there_sends_entered_once_then_delays_retry`
- `refinery_candock_non_roger_13_without_sidecheck_skips_enter_burst`
- `refinery_candock_side_check_allows_busy_requester_with_different_navcom`
- `miner_dock_first_0x16_sets_facing_without_prepare`
- `miner_dock_synced_0x16_can_prepare_from_accepted_cell`
- `miner_dock_second_0x16_requires_reaccepted_already_at_cell`
- `miner_dock_getdockcoord_cell_entry_sends_0x15`
- `miner_dock_contact_flag_adjacent_branch_sends_0x15_when_entered`
- `miner_dock_two_0x15_sources_ordered_by_tick_phase`
- `refinery_release_does_not_promote_waiter_until_own_mission_enter_timer_due`

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Accepted movement target is NW+(3,1), not `GetDockCoord` or `QueueingCell`. | `0x0043C2D0`; synthesis | mostly represented | `refinery_can_dock_queue_cell`, tests around accepted cell | preserve helper/name split | `war_miner_close_return_uses_accepted_cell_not_queueingcell` | do not rename/fold helpers into generic pad |
| Early `GetDockCoord` is a side-check against requester `+0x5A4` and controls non-ROGER `0x13` continuation. | `0x0043C2D0`; side-check report | missing/unchecked | `phase_mission_enter`; contact admission model | model `0x13` precheck if parity surface is implemented | `refinery_candock_non_roger_13_without_sidecheck_skips_enter_burst` | do not use side-check coordinate as move target |
| `0x12 == 1` movement path sends no `0x18/0x16` and Mission Enter retry is `14..16` frames. | `0x0043C2D0`; `0x004D9290`; Enter report | current phase returns to `MissionEnter` after arrival, but no explicit Enter delay found in scan | `phase_awaiting_accepted_cell`, `phase_mission_enter`, possible miner timer field | add mission-dispatch delay before the already-there recheck | `mission_enter_accepted_cell_recheck_waits_enter_rate_delay` | do not recheck CAN_DOCK every tick |
| `0x12 == 0x14` sends `0x18` then `0x16`; first `0x16` may be timer-only. | `0x0043C2D0`; `0x00737430` | `Linked`/`Pivoting` compress contact, pad, timer, and `0x15` | `phase_mission_enter`, `phase_linked`, `phase_pivoting` | split contact-entered, first-0x16 facing sync, and later `0x15` handoff | `miner_dock_first_0x16_sets_facing_without_prepare` | do not treat `0x16` return `1` as `0x15` sent |
| Later/already-synced `0x16` can send `0x15` from stopped accepted cell without `GetDockCoord` equality. | `0x00737430`; Drive report | current code snapshots to pad in `phase_linked` | `phase_linked`, pad occupancy bookkeeping | source-aware `0x16 -> 0x15`; avoid physical NW+2 precondition | `miner_dock_synced_0x16_can_prepare_from_accepted_cell` | do not force NW+(3,1)->NW+(2,1) move |
| PerCellProcess `0x15` is separate and includes GetDockCoord equality plus a contact-flag adjacent branch. | `0x00739EC0`; PerCellProcess/Drive reports | no explicit source split | future per-cell integration; `phase_linked` tests | keep per-cell source identity separate from radio source | `miner_dock_two_0x15_sources_ordered_by_tick_phase` | do not collapse all `0x15` as generic `Linked` |
| State-4 normal completion releases contact/pad and queues Harvest/Search, no waiter promotion. | synthesis; two-miner reports; Rust `phase_departing` scan | broadly represented | `phase_departing`, `RefineryDockContacts::release_contact`, two-miner tests | ensure waiter enters only on own due MissionEnter | `refinery_release_does_not_promote_waiter_until_own_mission_enter_timer_due` | do not implement refinery-side FIFO promotion |

## 7. Negative Facts / Do Not Do

- Do not force a physical move from accepted NW+(3,1) to stock `GetDockCoord` NW+(2,1).
- Do not use `GetDockCoord` as the accepted `0x12` target.
- Do not use `QueueingCell=4,1` as the accepted `0x12` target.
- Do not collapse `HELLO`, `CAN_DOCK`, `MOVE_TO_CELL`, `ENTER_DOCK`, `0x16`, `0x15`, and unload start into one Rust phase.
- Do not start unload merely because movement to accepted cell completed.
- Do not treat `0x16` return `1` as proof that `0x15` was sent.
- Do not require `GetDockCoord` equality before every possible `0x15`.
- Do not treat `UnitClass::PerCellProcess @ 0x00739EC0` as the mission-7 dispatch handler.
- Do not poll busy/waiting `CAN_DOCK` every tick; Mission Enter retry is timer-gated.
- Do not model normal stock unload completion as reciprocal `+0x2E4` release or depot-style queue promotion.
- Do not hide snapshot-vs-physical-position drift by setting only `snap.rx/snap.ry` without naming which gamemd field/source is being matched.

## 8. Remaining Uncertainty

- Exact first `0x15` source in every retail replay frame remains runtime-sensitive: later/aligned `0x16`, PerCellProcess `GetDockCoord`, or contact-flag adjacent-building branch.
- Exact facing/timer frame count from first unsynced `0x16` to `RateTimer::Current(+0x388) == 0x4000` for each relevant unit `Rot` remains a focused timing follow-up.
- Exact live equality frequency for requester `+0x5A4 == GetDockCoord CellClass*` in the early side-check requires runtime watchpoints.
- Rust has useful current tests, but the scan did not run tests and did not prove the current implementation already satisfies all contract items.

## 9. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Canonical stock refinery dock model | verified from docs | synthesis doc | none for contract input |
| Building `0x0E` accepted payload and side-check | spot-checked | Ghidra `0x0043C2D0` | exact runtime frequency of side-check equality |
| Unit `0x16` first/later split | spot-checked | Ghidra `0x00737430` | exact timer duration by Rot |
| Mission Enter retry | spot-checked/docs | Ghidra `0x004D9290`, Enter report | Rust implementation detail |
| PerCellProcess source split | spot-checked/docs | Ghidra `0x00739EC0`, PerCell reports | exact first-source runtime winner |
| Rust surfaces/tests | focused scan | Codegraph + `rg` + file reads | implementation contract/fix pass |
| INI liveness | verified from docs and focused grep | `rulesmd.ini`, `artmd.ini` | none for stock |

## Sources

- `docs/research/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/FOOTCLASS_MISSION_ENTER_0X0E_REPEAT_TIMING_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/BUILDING_RECEIVE_RADIO_0E_GETDOCKCOORD_SIDE_CHECK_GHIDRA_REPORT.md`
- `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`
- `docs/research/REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`
- Ghidra read-only decompile: `BuildingClass__Receive_Radio @ 0x0043C2D0`
- Ghidra read-only decompile: `UnitClass__Receive_Radio @ 0x00737430`
- Ghidra read-only decompile: `FootClass__Mission_Enter @ 0x004D9290`
- Ghidra read-only decompile: `UnitClass__PerCellProcess @ 0x00739EC0`
- Rust scan: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/components.rs`
- INI scan: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`

