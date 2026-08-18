# Miner Dock Queue: Two Chrono Miners, One Refinery

Date: 2026-05-20  
Trace slot: 3  
Scenario: Two same-owner standard YR Chrono Miners return to the same owned refinery with cargo. The first miner docks and unloads; the second miner targets/queues without entering the occupied pad or building footprint, waits, then takes over when the first miner departs.

> **Correction 2026-05-21 - stock DockUnload exit**
>
> Any queue-takeover claims below that depend on `ReleaseDockedHarvester`
> clearing the dock/contact state at normal stock departure are superseded.
> Stock GAREFN/NAREFN completion uses zero-link `Mission_Deploy_Building`
> state 4, with `+0x6D1` clear and conditional radio/contact cleanup. The
> `ReleaseDockedHarvester` model remains only for nonzero reciprocal-link
> release contexts.
>
> **Correction 2026-05-22 - contact saturation and close-return trace**
>
> `CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md`
> and `CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`
> supersede several queue assumptions below. Receiver-side full `HELLO(0x02)`
> returns `NEGATORY(10)` and does not evict refinery contacts; sender-side HELLO
> eviction only evicts the sender's old `Contacts[0]`. `0x17` is not the normal
> stock ore-refinery busy reply. `QueueingCell=4,1` is fallback/staging data after
> close HELLO cannot proceed or the return target is too far; the accepted
> `0x0E` cell remains hardcoded NW `+(3,1)`.

> **Repo-status supersession 2026-05-25 - return threshold**
>
> Any remaining notes below that describe current Rust using a hardcoded
> `CHRONO_INBOUND_WARP_THRESHOLD_CELLS` are stale. Current Rust reads
> `ChronoHarvTooFarDistance` for the CMIN close/far return split. Queue/contact
> conclusions should be read separately from that old threshold status.

## Scope

Only this concrete two-miner/one-refinery contention path was traced. Adjacent miner issues are listed at the end but not expanded.

Ghidra use was read-only. No program edits, labels, comments, renames, structs, or saves were made.

## Sources

- `ini/rulesmd.ini`: `CMIN` has `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=20`, `Teleporter=yes`; `GAREFN` and `NAREFN` have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`.
- `ini/artmd.ini`: `GAREFN` and `NAREFN` have `Foundation=4x3`, `QueueingCell=4,1`.
- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`: active YR radio chain and corrections.
- `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`: Mission_Enter / refinery queue path audit.
- Read-only Ghidra spot check:
  - `BuildingClass__Receive_Radio`: case `0x0E` checks `Type+0x16B3` / `Type+0x16BC`, computes queue cell as `GetMapCell().X + 3`, `GetMapCell().Y + 1`, sends `0x12`, then `0x18`, then `0x16`.
  - `UnitClass__Mission_Harvest`: standard active harvester FSM reads `Harvester=yes` at `TechnoType+0xE0E`, selects docks, and transitions to `Set_Mission(7, 0)` for enter/dock.
  - `RadioClass__FindDockSlot`: scans `Contacts[]` at `+0xE4/+0xE8`.

These references are active in standard YR: the code paths are gated by `DockUnload=yes`, `Refinery=yes`, `Harvester=yes`, and standard `GAREFN`/`NAREFN`/`CMIN` rules data, not TS-only fog/weeder/slave legacy.

## Pipeline

Full CMIN cargo -> find same-owner refinery -> inbound chrono return -> dock admission / queue -> first miner unloads -> first miner departs -> queued miner gets the dock -> second miner unloads.

## Stage Trace

### Stage 1 - Scenario Data

Input: two same-owner `CMIN`, one owned `GAREFN` or `NAREFN`, both miners carry cargo.

Rust:
- `CMIN` state is represented by `MinerKind::Chrono`, `MinerState`, cargo vector, and `RefineryDockPhase`.
- Refinery dock capacity is modeled by `DockReservations.occupied` with one occupant per refinery and `queues` for waiters in `src/sim/miner/miner_dock.rs:18`.

gamemd:
- `CMIN`: `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=20`, `Teleporter=yes`.
- `GAREFN` / `NAREFN`: `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`.
- `NumberOfDocks=1` sizes `Contacts[]`; `RadioClass__FindDockSlot` scans the contact array.

Verdict: UNCHECKED. The INI values match the scenario, but this trace did not run the Rust parser and gamemd loader side by side to prove literal runtime field equality.

### Stage 2 - Return Trigger And Refinery Selection

Rust:
- Full miner calls `begin_return`, then `find_nearest_refinery`, writes `reserved_refinery`, and transitions to `ReturnToRefinery` in `src/sim/miner/miner_system.rs:859`.

gamemd:
- `UnitClass__Mission_Harvest` state 2/3 finds a dock using the unit type's dock list and standard refinery flags, then transitions to Mission Enter (`Set_Mission(7, 0)`).

Verdict: UNCHECKED. Both paths select a refinery, but exact same-owner iteration ordering and concrete chosen refinery id were not computed in both engines for a fixed map/entity ordering.

### Stage 3 - Chrono Inbound Destination Before Dock Admission

Rust:
- If the Chrono Miner is farther than `CHRONO_INBOUND_WARP_THRESHOLD_CELLS`, `begin_return` warps directly to the refinery pad cell before any `DockReservations.try_reserve` call.
- Evidence: `src/sim/miner/miner_system.rs:884` chooses the chrono branch, `src/sim/miner/miner_system.rs:889` resolves `pad`, and `src/sim/miner/miner_system.rs:897` issues the teleport.
- The first dock reservation attempt happens later in `phase_approach` at `src/sim/miner/miner_dock_sequence.rs:416`.

gamemd:
- The active standard YR dock path uses Mission_Harvest -> Mission_Enter -> refinery radio admission. `BuildingClass__Receive_Radio` case `0x0E` establishes/queries the dock link and returns a receiver target cell before the harvester proceeds through the dock choreography.
- Correction 2026-05-21: standard DockUnload occupied/no-slot is not a simple hard NEGATORY. `HELLO(0x02)` can reject when `Contacts[]` is full, but case `0x0E` can still send `0x12` for the hardcoded `GetMapCell()+(3,1)` target and return `1` without `0x18/0x16`.

Output difference:
- Rust can materialize the second Chrono Miner directly on the occupied pad / refinery footprint before it knows the dock is unavailable.
- gamemd gates the dock approach through radio/Contacts before the harvester enters the dock choreography.

Verdict: FAIL. This directly violates the scenario requirement that the second miner queue without entering the occupied pad or footprint.

### Stage 4 - Queue Cell Computation

Rust:
- `refinery_queue_cell` uses parsed art `QueueingCell=` when available, so standard `GAREFN`/`NAREFN` use `(rx+4, ry+1)` from `QueueingCell=4,1`.
- Evidence: `src/sim/miner/miner_dock_sequence.rs:65` and `src/sim/miner/miner_dock_sequence.rs:72`.

gamemd:
- Read-only decompile of active `BuildingClass__Receive_Radio` case `0x0E` computes the accepted DockUnload/Weeder cell as `GetMapCell().X + 3`, `GetMapCell().Y + 1`.
- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` records that `QueueingCell=` is stored but not read in `Receive_Radio` case `0x0E`.
- 2026-05-22 contact-saturation research verifies that `QueueingCell=4,1` is used by `UnitClass::Mission_Harvest` fallback after close HELLO cannot proceed or when the return target is too far; it is not the already-accepted `0x0E` dock cell.

Output difference:
- If Rust `rx,ry` and gamemd `GetMapCell()` are the same placement anchor, Rust waits one cell farther east than gamemd.
- This trace did not independently prove the anchor equivalence, so the observed world-cell delta remains unresolved.

Verdict: UPDATED. Do not compare Rust's fallback `QueueingCell=4,1` directly against the accepted `0x0E` cell. The relevant parity question is whether Rust uses `QueueingCell=4,1` only for fallback/staging and NW `+(3,1)` for accepted admission.

### Stage 5 - Second Miner Enqueues While First Occupies Dock

Rust:
- `DockReservations.try_reserve` returns `false` when an occupant exists and pushes the second miner into a FIFO `VecDeque` if absent.
- Evidence: `src/sim/miner/miner_dock.rs:30`, `src/sim/miner/miner_dock.rs:36`, `src/sim/miner/miner_dock.rs:38`.
- `phase_approach` sets `dock_queued = true` and only issues a move to the queue cell when not adjacent/at queue.
- Evidence: `src/sim/miner/miner_dock_sequence.rs:426`, `src/sim/miner/miner_dock_sequence.rs:429`.

gamemd:
- `Contacts[]` capacity is one for standard refineries. Receiver-side full `HELLO(0x02)` returns `NEGATORY(10)` and does not evict the current refinery contact.
- Sender-side HELLO eviction is real, but evicts the sender's own old `Contacts[0]`, not the refinery receiver's current contact.
- `BuildingClass__Receive_Radio` case `0x08` returns `ROGER(1)` for stock GAREFN/NAREFN, not `0x17 QUEUED`; `0x17` belongs to factory/repair/bunker-style paths.
- `BuildingClass__Receive_Radio` case `0x0E` does not hard-reject solely because the refinery contact slot is full; it can still send `0x13/0x12` and return `1`.

Verdict: UPDATED / PARTIAL. The older broad "radio admission / queue" description is too vague. Use the 2026-05-22 contact-saturation report for exact full-contact behavior; this trace still does not compute the exact two-miner per-tick wait position.

### Stage 6 - First Miner Unloads

Rust:
- After reservation, `phase_linked` moves to `Pivoting`; `phase_unloading` drains cargo slots and keeps the dock held through `DepositCooldown`.
- Evidence: `src/sim/miner/miner_dock_sequence.rs:436`, `src/sim/miner/miner_dock_sequence.rs:518`, `src/sim/miner/miner_dock_sequence.rs:602`.

gamemd:
- `Mission_Deploy_Building` handles the refinery drain on the unit side; `HarvesterDumpRate * 900` gives the 14.4-frame dump gate, and the dock link remains active during unload.

Verdict: UNCHECKED. Timing and credit parity were not recomputed here because this slot focuses on queue contention, not the whole deposit formula.

### Stage 7 - First Miner Departs Through A Blocked Queue Cell

Rust:
- If the second miner is waiting at the queue cell, `phase_departing` still issues the exit move and then sets `movement_target.bypass_grid = true`.
- Evidence: `src/sim/miner/miner_dock_sequence.rs:677`, `src/sim/miner/miner_dock_sequence.rs:693`, `src/sim/miner/miner_dock_sequence.rs:730`, `src/sim/miner/miner_dock_sequence.rs:733`.
- `detect_deferred_cell_check` skips occupancy checks entirely when `bypass_grid` is true.
- Evidence: `src/sim/movement/movement_occupancy.rs:91`, `src/sim/movement/movement_occupancy.rs:116`.
- Existing regression test intentionally models this as "briefly overlapping any waiting miner" behavior.

gamemd:
- Updated 2026-05-22: stock exit is zero-link `Mission_Deploy_Building` state 4, not normal `ReleaseDockedHarvester`. The Rust comment still identifies the relevant parity issue: gamemd handles blocked queue-cell handoff via protocol coordination and radio/contact cleanup instead of disabling occupancy for the exiting mover.

Output difference:
- Rust can visibly push through or overlap the waiting miner at the queue cell.
- gamemd coordinates the queue/contact handoff instead of disabling occupancy for the exiting mover.

Verdict: FAIL. This is player-visible in the exact two-miner contention scenario.

### Stage 8 - Handoff Timing

Rust:
- The dock is released only after the first miner reaches its exit target: `if !moving && at_exit && !teleporting`, then `dock_reservations.release(ref_sid)`.
- Evidence: `src/sim/miner/miner_dock_sequence.rs:738`, `src/sim/miner/miner_dock_sequence.rs:742`.
- `DockReservations.release` immediately inserts the next queued miner as the new occupant.
- Evidence: `src/sim/miner/miner_dock.rs:49`, `src/sim/miner/miner_dock.rs:55`, `src/sim/miner/miner_dock.rs:56`.

gamemd:
- Superseded 2026-05-21: the older stage table placed field/contact clearing inside normal `ReleaseDockedHarvester` exit. Stock zero-link DockUnload instead exits through `Mission_Deploy_Building` state 4 with `+0x6D1` clear and conditional radio/contact cleanup.
- That means the queue handoff is tied to exit initiation, not to the moment the previous miner has fully reached Rust's exit target.

Output difference:
- Rust delays the second miner's takeover until the first miner arrives at the exit cell.
- gamemd releases/clears the dock contact as part of the departure transition.

Verdict: FAIL. Exact tick delta was not computed, but the ordering boundary differs.

### Stage 9 - Promotion Order

Rust:
- FIFO promotion is explicit: `VecDeque::pop_front`.
- Evidence: `src/sim/miner/miner_dock.rs:52`.

gamemd:
- Contacts/free-slot behavior is verified, but this trace did not compute queue-vector ordering for the exact two-harvester case beyond the first available slot.

Verdict: UNCHECKED. Likely correct for two miners, but exact equality was not computed.

### Stage 10 - Final Screen Result

Expected gamemd screen result:
- First miner docks and unloads.
- Second miner waits outside, not on the occupied pad or footprint.
- First miner departs; queued miner takes over without visible overlap.

Rust screen result from code inspection:
- The second Chrono Miner can warp onto the pad before reservation.
- During departure with a waiting miner, the first miner can bypass occupancy through the queue cell.
- Handoff is delayed until first miner reaches the exit target.

Verdict: FAIL.

## Verdict Tally

PASS: 0  
FAIL: 4  
UNCHECKED: 6  
NOT-IMPLEMENTED: 1

NOT-IMPLEMENTED item: gamemd-style queue-cell coordination / side-step handoff during refinery exit is not modeled; Rust uses `bypass_grid` as a minimal escape hatch instead.

## Top Player-Visible Findings

1. FAIL - Stage 3: second Chrono Miner can materialize on the occupied pad/footprint before dock admission; Rust `src/sim/miner/miner_system.rs:884`; gamemd evidence: active `UnitClass__Mission_Harvest` + `BuildingClass__Receive_Radio` case `0x0E` radio admission/queue workflow.
2. FAIL - Stage 7: departing first miner can overlap/push through the waiting miner because exit movement bypasses occupancy; Rust `src/sim/miner/miner_dock_sequence.rs:733`; gamemd evidence: stock zero-link state-4 exit uses mission/radio contact cleanup, not occupancy bypass.
3. SUPERSEDED 2026-05-21 - Stage 8: this queued-takeover finding depended on normal exit clearing DockedIn/DockLink/Contacts inside `ReleaseDockedHarvester`. Current stock-path evidence uses zero-link `Mission_Deploy_Building` state 4 instead, so this row needs a fresh trace before implementation work.
4. NOT-IMPLEMENTED - Stage 7: no gamemd-equivalent queued-miner side-step / radio handoff; Rust comment at `src/sim/miner/miner_dock_sequence.rs:716` describes the missing protocol coordination; gamemd evidence: active refinery radio protocol and Contacts[] handoff.

## Adjacent Findings

- The queue-cell coordinate path is now split by context: Rust should honor `QueueingCell=4,1` for fallback/staging but use hardcoded NW `+(3,1)` for accepted `0x0E` admission. The remaining trace gap is exact two-miner per-tick handoff timing.
- The return-warp threshold remains hardcoded in Rust and was not re-verified in this slot.
- Deposit timing/credit parity was intentionally not re-traced here.

## Status

COMPLETE for this slot. No Rust, INI, in-repo docs, or other files were modified.
