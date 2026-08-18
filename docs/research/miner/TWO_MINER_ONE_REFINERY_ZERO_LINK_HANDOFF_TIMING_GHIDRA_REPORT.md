# Two Miner One Refinery Zero-Link Handoff Timing - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x0043C2D0`, `0x0065A820`, `0x0065A970`, `0x0065ACB0`, `0x0065AD90`, `0x0065ADF0`, `0x0065AE30`, `0x006F4AB0`, `0x004D9290`, `0x00741970`, `0x00739EC0`, `0x00737430`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Standard YR two stock harvesters contending for one stock refinery after the first miner reaches the stock zero-link `Mission_Deploy_Building` state-4 exit. This report verifies the static ordering of state-4 cleanup, radio/contact release, `+0x418` clearing, and retry admission gates.  
**Non-Scope:** Frame-perfect multi-object tick order, exact visible collision/side-step behavior while the old miner still occupies the pad cell, slave miners, Bunker reciprocal-link release, service depots, aircraft docks, and runtime pathfinder retry timing.  
**Confidence:** High for static ordering inside decompiled functions; Medium for the two-miner handoff model because exact frame/tick promotion needs runtime debugger observation.  
**Active in YR:** Yes. The path is gated by stock `[CMIN]`/`[HARV] Harvester=yes`, `[GAREFN]`/`[NAREFN] DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.

## 1. Overview

The normal stock `CMIN/HARV -> GAREFN/NAREFN` completion path is the zero-`+0x2E4` state-4 branch in `UnitClass::Mission_Deploy_Building`, not `BuildingClass::ReleaseDockedHarvester`. State 4 first waits for the refinery production/door anim pointer `building+0x57C` to clear, then clears the harvester unload-active byte `unit+0x6D1`, assigns Harvest mission `0x0A`, optionally sends radio `BREAK(0x03)` through the first contact, and only then queues the next mission.

For two miners, dock availability is therefore a radio/contact retry problem. The first miner's contact slot is released by the state-4 `BREAK(0x03)` cascade when that conditional branch fires; the second miner is not promoted by a persistent binary FIFO. The exact frame on which the second miner's next `HELLO`/`CAN_DOCK` retry observes the free slot is deferred to runtime debugging because it depends on object iteration order and mission scheduling outside this static slice.

## 2. Class Layout / Key Offsets

| Field / helper | Offset / address | Static meaning in this slice | Active in YR |
|---|---:|---|---|
| `RadioClass::Contacts.data` | `+0xE4` | Sparse fixed-capacity contact pointer array | Yes |
| `RadioClass::Contacts.Capacity` | `+0xE8` | Contact array size; stock refinery capacity becomes `NumberOfDocks=1` | Yes |
| `RadioClass::FindDockSlot` | `0x0065AD90` | Returns contact index for a target or `-1` | Yes |
| `FUN_0065ADF0` | `0x0065ADF0` | Returns true when a contact slot is free or already equals the target | Yes |
| `PathType__Has_Valid_Steps` | `0x0065AE30` | Misnamed in this context; returns true when any radio contact slot is non-null | Yes |
| `unit+0x418` / `building+0x418` | byte at `+0x418` | Radio entered/contact byte set by `0x18`, cleared by `0x19` | Yes |
| `unit+0x5A4` | `+0x5A4` | Destination/target field read during state-4 branch selection | Yes |
| `unit+0x6D1` | byte at `+0x6D1` | Harvester unload initialized/active byte; state 4 clears it | Yes |
| `unit+0xBC` | `+0xBC` | Mission substate; state 3 drains, state 4 exits | Yes |
| `unit+0x2E4` | `+0x2E4` | Conditional reciprocal link; stock refinery path normally zero | Conditional, not normal stock unload |
| `building+0x57C` | `+0x57C` | State-4 wait guard for refinery `ProductionAnim`/door anim pointer | Yes |
| `BuildingType+0x16B3` | `+0x16B3` | `DockUnload=yes`; radio `0x15` queues sender mission `0x10` | Yes |
| `BuildingType+0x16BB` | `+0x16BB` | `Refinery=yes`; state-4 `+0x57C` wait and state-3 close anim | Yes |
| `BuildingType+0x1780` | `+0x1780` | `NumberOfDocks`; feeds contact capacity during construction | Yes |

## 3. Core Logic

### 3.1 Static state-4 order

When `UnitClass::Mission_Deploy_Building @ 0x0073D630` is on the stock zero-link harvester path and substate is `4`, the verified order is:

1. Re-find the refinery by current cell plus the west-neighbor lookup global, then `Look_up_building_in_cell`.
2. If a building exists, `BuildingType+0x16BB != 0`, and `building+0x57C != 0`, return `1` immediately. No contact clear happens on this wait tick. Evidence: `0x0073E1D5`, `0x0073E1DF`, `0x0073E1EA -> 0x0073E5B1`.
3. Clear `unit+0x6D1 = 0`. Evidence: `0x0073E1F6`.
4. In the normal branch, call unit vtable `+0x1E8` with `0x0A, 0`, assigning Harvest mission. Evidence: `0x0073E24F..0x0073E254`.
5. Call unit vtable `+0x200`; if it returns false, skip the radio break and mission queue call and go to the timer epilogue. Evidence: `0x0073E25E`, `0x0073E264`, `0x0073E266`.
6. If `PathType__Has_Valid_Steps @ 0x0065AE30` reports any contact slot, send `BREAK(0x03)` with unit vtable `+0x274` (`Transmit_Radio_ToFirst`). Evidence: `0x0073E26A`, `0x0073E26F`, `0x0073E275`, `0x0073E279`.
7. Call unit vtable `+0x1EC` to queue/advance the mission. Evidence: `0x0073E27F..0x0073E283`.
8. Return through the mission timer/random epilogue. Evidence: `0x0073E289..0x0073E2B7`.

The contact release boundary is therefore after `+0x6D1` clear and after Harvest mission assignment, but before the explicit `+0x1EC` mission queue call completes. It is not after a queue-cell drive and not after `ReleaseDockedHarvester`.

### 3.2 What `BREAK(0x03)` clears

The state-4 radio clear is `Transmit_Radio_ToFirst(3)`:

1. `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0` reads `Contacts[0]`; if null, returns `0`.
2. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` handles message `3` by clearing every sender-side contact slot equal to the target before dispatching to the target. Evidence: `0x0065A9A8..0x0065A9BE`, `0x0065A9C9`.
3. The target refinery receives `BuildingClass::Receive_Radio(3)`, calls `GrandOpening`, then delegates to `TechnoClass::Receive_Radio(3)`. Evidence: `0x0043C2D0` case `3`.
4. `TechnoClass::Receive_Radio(3)` conditionally sends `0x19` before falling through to base `RadioClass::Receive_Radio(3)` if both participants have `+0x418` set. Evidence: `0x006F4AB0` case `3`.
5. `TechnoClass::Receive_Radio(0x19)` clears the receiver's `+0x418` byte and propagates `0x19` once; recursion stops when the peer byte is already clear. Evidence: `0x006F4AB0` cases `0x18/0x19`.
6. Base `RadioClass::Receive_Radio(3)` then finds the sender in the receiver's `Contacts[]`, calls `ObjectClass::Receive_Radio` for side effects, nulls that slot, and returns `1`. Evidence: `0x0065A820` break branch.

Important ordering detail: the miner's contact slot is nulled before the refinery receives `BREAK`. The refinery's `+0x418`/miner `+0x418` clear cascade happens before the refinery's base `RadioClass` nulls its contact slot.

### 3.3 Admission / retry when the second miner is waiting

Static admission is not a FIFO promotion. The live primitives are:

- `NumberOfDocks=1` sizes the stock refinery contact array to one slot.
- `RadioClass::Receive_Radio(HELLO=0x02)` accepts only if a slot is free or the sender is already present; if full, it returns `NEGATORY(10)`. Evidence: `0x0065A820`.
- `FUN_0065ADF0` returns true when any slot is null or already equals the target. Evidence: `0x0065ADF0`.
- `BuildingClass::Receive_Radio(0x0E)` checks contact containment and free-slot eligibility; when the sender is accepted or can be linked, the building can transmit `HELLO(0x02)`, then later send `0x13`, hardcoded accepted cell `NW+(3,1)` via `0x12`, then `0x18`, then `0x16`. Evidence: `0x0043C8A4..0x0043C8C8`, `0x0043C9F5..0x0043CADB`.
- `FootClass::Mission_Enter @ 0x004D9290` re-sends `0x0E` on its tick and sends `BREAK(0x03)`/backs out when the response is not accepted and no `+0x418`-like entered byte preserves it.

What is statically provable: once the first miner's state-4 `BREAK` has nulled the refinery's contact slot, a later `HELLO`/`CAN_DOCK` retry from the second miner can pass the one-slot capacity gate. What is not statically provable: whether the second miner's successful retry happens in the same game frame after the first miner's state-4 call, the next frame, or a later retry frame.

### 3.4 Pad / on-dock occupancy boundary

The binary path does not expose a separate stock "pad occupancy map" equivalent to Rust's `on_pad`. The stock state-4 path clears radio/contact state and returns to Harvest scheduling without issuing `Force_Track(0x47)` or an explicit queue-cell exit command. The old miner's physical object still exists at its current cell until later movement/mission logic advances it.

Therefore:

- Contact availability changes during state-4 radio cleanup, before any verified physical move off the pad.
- `+0x6D1` is already clear before radio contact cleanup starts.
- `+0x418` clears during the conditional `0x19` cascade caused by `BREAK(0x03)`.
- The exact player-visible avoidance of pad overlap, side-step, or blocked queue-cell behavior is DEFERRED. It needs runtime debugger observation of object order, position, mission, path, and contact slots across frames.

## 4. INI Keys

| INI key | YR value | Base RA2 value | Static effect | Evidence |
|---|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | `NAREFN,GAREFN` | Allows Chrono Miner to choose stock refineries | `rulesmd.ini:7361`, `rules.ini:6038` |
| `[CMIN] Harvester` | `yes` | `yes` | Enables harvester unload FSM | `rulesmd.ini:7364`, `rules.ini:6041` |
| `[CMIN] Teleporter` | `yes` | `yes` | Chrono movement; relevant to runtime arrival timing, not state-4 contact clear | `rulesmd.ini:7396`, `rules.ini:6070` |
| `[HARV] Dock` | `NAREFN,GAREFN` | `NAREFN,GAREFN` | Allows War Miner to choose stock refineries | `rulesmd.ini:8225`, `rules.ini:5934` |
| `[HARV] Harvester` | `yes` | `yes` | Enables same unload FSM | `rulesmd.ini:8228`, `rules.ini:5937` |
| `[GAREFN] DockUnload` | `yes` | `yes` | Radio `0x15` sends sender to mission `0x10` | `rulesmd.ini:11726`, `rules.ini:8558` |
| `[GAREFN] Refinery` | `yes` | `yes` | State-4 checks `building+0x57C` before exit | `rulesmd.ini:11727`, `rules.ini:8559` |
| `[GAREFN] NumberOfDocks` | `1` | `1` | One contact slot | `rulesmd.ini:11729`, `rules.ini:8561` |
| `[NAREFN] DockUnload` | `yes` | `yes` | Same DockUnload handoff | `rulesmd.ini:12519`, `rules.ini:8601` |
| `[NAREFN] Refinery` | `yes` | `yes` | Same state-4 wait gate | `rulesmd.ini:12520`, `rules.ini:8602` |
| `[NAREFN] NumberOfDocks` | `1` | `1` | One contact slot | `rulesmd.ini:12521`, `rules.ini:8603` |
| `artmd.ini [GAREFN]/[NAREFN] Foundation` | `4x3` | `4x3` | Placement geometry; not the state-4 contact boundary | `artmd.ini:1709`, `1766`; `art.ini:1100`, `1157` |
| `artmd.ini [GAREFN]/[NAREFN] QueueingCell` | `4,1` | `4,1` | INI wait-cell data; `Receive_Radio(0x0E)` hardcodes accepted cell `NW+(3,1)` instead | `artmd.ini:1716`, `1773`; `art.ini:1107`, `1164` |

## 5. Integration Points

| Function | Role in this slice | Verified details |
|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | Stock unload state machine and state-4 handoff | Zero-`+0x2E4` state 4 waits on `building+0x57C`, clears `+0x6D1`, assigns mission `0x0A`, optionally sends `BREAK(3)`, then queues mission |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | Refinery admission and pad-arrival receiver | Case `0x0E` performs admission choreography; case `0x15` queues sender mission `0x10`; case `3` delegates break cleanup |
| `RadioClass::Receive_Radio @ 0x0065A820` | Base HELLO/BREAK contact storage | HELLO rejects full Contacts; BREAK nulls receiver slot matching sender |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | Sender-side synchronous dispatch | BREAK clears sender slot before target receive; HELLO can evict sender's slot 0 when sender slots are full |
| `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0` | State-4 `+0x274` target selection | Sends to `Contacts[0]` only; returns `0` if `Contacts[0]` is null |
| `FUN_0065ADF0 @ 0x0065ADF0` | Free-slot test | True when a slot is null or already equals target |
| `PathType__Has_Valid_Steps @ 0x0065AE30` | Contact-present test | Walks Contacts and returns true if any slot is non-null |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `+0x418` state | `0x18` sets, `0x19` clears, `0x03` can trigger `0x19` before base BREAK cleanup |
| `FootClass::Mission_Enter @ 0x004D9290` | Retry driver | Re-sends `0x0E`; on rejection with no preserve condition sends `BREAK` and clears destination |
| `TechnoClass::Set_Destination @ 0x00741970` | Harvester destination/admission sender | Sends `HELLO` and `CAN_DOCK` in live return path; broad function touched, not exhausted |
| `UnitClass::PerCellProcess @ 0x00739EC0` | Pad-arrival sender | Sends `0x15` to refinery when the unit reaches the pad/accepted cell |
| `UnitClass::Receive_Radio @ 0x00737430` | Unit-side radio responses | Handles `0x16` and can send `0x15`; broad function touched, not exhausted |

## 6. Current Rust Implementation Status

The current Rust surfaces to treat as risky are:

- `src/sim/miner/miner_dock.rs`: `RefineryDockContacts` now mirrors contacts, waiting retry queue, `contact_entered`, and `on_pad`. `DockReservations` still exists as an older FIFO compatibility surface.
- `src/sim/miner/miner_dock_sequence.rs`: `phase_approach`, `phase_mission_enter`, `phase_unloading`, `phase_deposit_cooldown`, and `phase_departing` model the handoff. `phase_departing` releases `on_pad` and contact immediately and does not issue a stock exit move.
- `src/sim/miner/miner_system.rs`: `begin_return`, chrono staging, and `refinery_dock_cell` decide how the second Chrono Miner approaches the refinery before the contact slot is available.
- `src/sim/miner/miner_tests.rs`: queue/contact tests around `dock_wait_grants_reservation_when_free`, `queued_miner_enters_after_contact_and_pad_are_released`, `departing_handoff_ignores_blocked_queue_cell`, and death/invalid-refinery tests encode current expected behavior.

Observed status:

- Rust now correctly separates stock zero-link state-4 cleanup from `ReleaseDockedHarvester`/`Force_Track(0x47)` effects.
- Rust has a deterministic `waiting_retry_queue`; gamemd has no verified persistent FIFO in this slice. It retries through radio/contact state and object tick order.
- Rust gates second-miner entry on both contact and `on_pad` being clear. Static binary evidence proves contact release during state 4; it does not prove a separate pad-occupancy gate equivalent to Rust's `on_pad`.
- Rust uses art `QueueingCell=4,1` for some staging/wait helpers while `BuildingClass::Receive_Radio(0x0E)` hardcodes accepted cell `NW+(3,1)`. The code has a separate `refinery_can_dock_queue_cell`, but chrono staging still starts from `QueueingCell`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock state-4 wait on `building+0x57C` | verified | `0x0073E1D5..0x0073E1EA` | none |
| State-4 `unit+0x6D1` clear | verified | `0x0073E1F6` | none |
| State-4 Harvest mission assignment | verified | `0x0073E24F..0x0073E254` | exact mission enum naming outside scope |
| State-4 contact-present guard | verified | `0x0073E26A`, `0x0065AE30` | vtable `+0x200` semantic only touched |
| State-4 radio `BREAK(3)` | verified | `0x0073E275..0x0073E279`, `0x0065ACB0` | runtime whether branch always fires for stock normal exit |
| Sender-side contact clear on BREAK | verified | `0x0065A970`, assembly around `0x0065A9A8..0x0065A9BE` | none |
| Receiver-side contact clear on BREAK | verified | `0x0065A820` | none |
| `+0x418` clear cascade | verified | `0x006F4AB0` cases `3` and `0x19` | exact initial building `+0x418` value at every edge case deferred |
| HELLO full-slot rejection | verified | `0x0065A820` | runtime retry cadence deferred |
| Free-slot helper | verified | `0x0065ADF0` | none |
| Contact lookup helper | verified | `0x0065AD90` | none |
| Building case `0x0E` accepted-cell choreography | touched-not-exhausted | `0x0043C8A4..0x0043CADB` | full branch proof for full-slot non-contact response values |
| Building case `0x15` mission handoff | verified | `0x0043C2D0` case `0x15`; audit log correction | none |
| `FootClass::Mission_Enter` retry cadence | touched-not-exhausted | `0x004D9290` | exact frame timing by object order |
| `UnitClass::PerCellProcess` pad arrival | touched-not-exhausted | `0x00739EC0` | exact same-frame interaction with second miner |
| `TechnoClass::Set_Destination` | touched-not-exhausted | `0x00741970` | broad path; only admission-relevant branches sampled |
| Prior two-miner trace ReleaseDockedHarvester assumption | conflict-needs-resolution | old trace correction banner plus current `0x0073D630` | update/verify old trace if it is kept as canonical |
| Current Rust `RefineryDockContacts` | touched-not-exhausted | `src/sim/miner/miner_dock.rs` scan | compare against runtime debugger trace |
| Current Rust `phase_departing` | touched-not-exhausted | `src/sim/miner/miner_dock_sequence.rs:879` | acceptance tests after runtime frame trace |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which investigation mode applies? -> coverage-map, because static code proves ordering inside functions but not frame-perfect two-object tick order.` (evidence: task scope plus runtime-dependent retry)
- `[RESOLVED] OQ-02 - Is normal stock handoff `ReleaseDockedHarvester`? -> No; stock normal completion is zero-`+0x2E4` `Mission_Deploy_Building` state 4.` (evidence: `0x0073D630`, `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 - Does state 4 wait before clearing contact? -> Yes, it returns while `Refinery=yes` and `building+0x57C != 0`.` (evidence: `0x0073E1D5..0x0073E1EA`)
- `[RESOLVED] OQ-04 - When is `unit+0x6D1` cleared? -> Before Harvest mission assignment and before radio BREAK.` (evidence: `0x0073E1F6`, `0x0073E24F`, `0x0073E275`)
- `[RESOLVED] OQ-05 - When is the miner-side Contacts[] cleared? -> In `Transmit_Radio_Impl(3)`, before the refinery receives BREAK.` (evidence: `0x0065A970`, `0x0065A9A8..0x0065A9C9`)
- `[RESOLVED] OQ-06 - When is the refinery-side Contacts[] cleared? -> In base `RadioClass::Receive_Radio(3)` after Building/Techno case-3 handling.` (evidence: `0x0065A820`, `0x0043C2D0`, `0x006F4AB0`)
- `[RESOLVED] OQ-07 - When does `+0x418` clear? -> During the conditional `0x19` cascade triggered by TechnoClass case `3`, before base receiver-side contact nulling.` (evidence: `0x006F4AB0`)
- `[RESOLVED] OQ-08 - Is `PathType__Has_Valid_Steps` a path guard here? -> No, it scans radio Contacts[] and returns true if any slot is non-null.` (evidence: `0x0065AE30`)
- `[RESOLVED] OQ-09 - Does `NumberOfDocks=1` matter? -> Yes; stock GAREFN/NAREFN have one contact slot, so a second miner cannot be simultaneously stored in refinery Contacts[].` (evidence: `rulesmd.ini:11729`, `12521`; `RadioClass::Set_Contact_Count @ 0x0065AE60`)
- `[RESOLVED] OQ-10 - Does the binary prove a persistent two-miner FIFO? -> No; static evidence shows contact slots and retry, not a persistent FIFO promotion list for stock refineries.` (evidence: `0x0065A820`, `0x0065ADF0`, `0x004D9290`)
- `[RESOLVED] OQ-11 - What happens to old trace Stage 8? -> The ReleaseDockedHarvester-based handoff is superseded for stock normal exit; replace it with zero-link state-4 BREAK/contact cleanup.` (evidence: old trace correction banner; `0x0073D630`)
- `[RESOLVED] OQ-12 - Is `QueueingCell=4,1` the accepted cell sent by the binary? -> No; case `0x0E` hardcodes `NW+(3,1)` for `0x12` accepted-cell payload.` (evidence: `0x0043CA8D..0x0043CAB8`; `artmd.ini:1716`, `1773`)
- `[RESOLVED] OQ-13 - Does state 4 issue `Force_Track(0x47)` or queue-cell exit movement? -> No static evidence in the zero-link state-4 path; those belong to conditional reciprocal-link release contexts.` (evidence: `0x0073D630`; `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-14 - Is the Rust surface high risk? -> Yes: `miner_dock.rs`, `miner_dock_sequence.rs`, `miner_system.rs`, and `miner_tests.rs` encode the abstraction that must match this handoff.` (evidence: Rust scan)
- `[RESOLVED] OQ-15 - Does state 4 release contact before or after mission queue call? -> Before `+0x1EC` mission queue/advance call when the contact-present branch fires.` (evidence: `0x0073E275`, `0x0073E27F..0x0073E283`)
- `[DEFERRED] OQ-16 - Does the second miner retry and enter in the same rendered frame as the first miner's state-4 contact clear?` (category: `needs-runtime-debugger`; reason: object iteration order and mission scheduler order are not fully derivable from this static slice; next-step-if-pursued: watch both miners' mission/substate/contact arrays over frames around first state-4 exit)
- `[DEFERRED] OQ-17 - Does binary gameplay allow second-miner radio acceptance while the first miner still physically occupies the pad cell?` (category: `needs-runtime-debugger`; reason: contact release is before verified movement, but visible collision/pad occupancy behavior depends on runtime path/movement checks; next-step-if-pursued: watch current cell, destination, locomotor moving flag, and contact slots for both miners)
- `[DEFERRED] OQ-18 - Is vtable `+0x200` always true for normal stock state-4 exit?` (category: `requires-different-system-context`; reason: branch is observed but the virtual method identity was not drained in this handoff-focused pass; next-step-if-pursued: resolve UnitClass vtable `+0x200` and test stock HARV/CMIN state-4 cases)
- `[DEFERRED] OQ-19 - What exact branch does `BuildingClass::Receive_Radio(0x0E)` take for a non-contact second miner while the slot is full?` (category: `needs-runtime-debugger`; reason: static decompile shows mixed retry/target code, but exact runtime values decide whether it returns accepted-looking `1` without contact; next-step-if-pursued: log return code, Contacts[], payload, and messages for second miner during full-slot wait)
- `[DEFERRED] OQ-20 - What exact side-step or avoidance motion prevents visible overlap in stock YR?` (category: `needs-runtime-debugger`; reason: no separate static FIFO/pad occupancy primitive was found; next-step-if-pursued: video/debug trace two Chrono Miners at one refinery with cell occupancy and locomotor destinations)
- `[DEFERRED] OQ-21 - Save/load mid-unload or pause effects on this handoff?` (category: `out-of-scope`; reason: task targets normal live contention; next-step-if-pursued: save at state 3, state 4 wait, and post-BREAK, then compare restored contacts)

Runtime watchpoints/events needed:

- First miner: `unit+0xBC`, `unit+0x6D1`, `unit+0x418`, `unit+0xE4` contact slot, current cell, mission, queued mission, movement destination.
- Refinery: `building+0xE4` contact slot, `building+0x418`, `building+0x57C`, mission, anim slot 8.
- Second miner: `unit+0xBC`, `unit+0x418`, contact slot, destination/accepted cell payload, current cell, mission, movement destination.
- Radio events: first miner `Transmit_Radio_ToFirst(3)`, refinery `Receive_Radio(3)`, `0x19` cascade, second miner `HELLO(2)` and `CAN_DOCK(0x0E)` return codes.
- Frame events: object iteration order for first miner state-4 call versus second miner `Mission_Enter` retry in the same game frame.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock normal exit is zero-link state 4, not `ReleaseDockedHarvester` | `0x0073D630`; branch reports | none observed in current `phase_departing` comments/logic | `src/sim/miner/miner_dock_sequence.rs:879` | Keep state-4 cleanup independent from forced track and queue-cell exit movement | Miner finishes unload with no `Force_Track(0x47)` and remains at pad cell during handoff tick | Do not reintroduce ReleaseDockedHarvester for normal stock cargo-empty exit |
| State 4 waits while refinery `+0x57C` is non-null | `0x0073E1D5..0x0073E1EA` | unchecked in this pass | `src/sim/miner/miner_dock_sequence.rs`, refinery anim state surfaces | If production/door anim is modeled, delay contact release until the anim guard clears | Last unload triggers close/production anim; contact stays held until guard clears | Do not release the next miner at cargo-empty if the binary would still wait on slot 8 |
| `unit+0x6D1` clears before radio BREAK | `0x0073E1F6`, `0x0073E275` | approximated by phase transition fields | `src/sim/miner/miner_dock_sequence.rs` | Clear unload-active/render override before or with contact handoff, not after a later exit drive | State leaves visual unload mode on handoff tick | Do not couple unload-active clear to reaching queue/exit cell |
| Contacts clear by `BREAK(0x03)` through radio, not FIFO promotion | `0x0065ACB0`, `0x0065A970`, `0x0065A820` | Rust adds deterministic `waiting_retry_queue` | `src/sim/miner/miner_dock.rs` | Treat Rust FIFO as a deterministic retry-order abstraction, not as proven binary storage | Two waiting miners retry in stable order but can be audited against runtime trace later | Do not claim binary has a persistent FIFO until runtime or deeper static proof exists |
| `+0x418` clears via `0x19` cascade during BREAK | `0x006F4AB0` | modeled as `contact_entered`; release ordering should be checked | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs` | Clear `contact_entered` as part of state-4 contact release | After first miner handoff, neither miner/refinery pair remains entered | Do not confuse `+0x418` with reciprocal `+0x2E4` |
| Second miner can only truly occupy Contacts[] after the first slot is null | `0x0065A820`, `NumberOfDocks=1` | likely matched by `hello_or_wait` capacity | `src/sim/miner/miner_dock.rs:42` | Keep one-slot admission for stock refineries | Second miner waits while first contact exists; enters after release | Do not allow two stock miners in refinery Contacts[] |
| Accepted cell sent by binary is `NW+(3,1)`, not art `QueueingCell=4,1` | `0x0043CA8D..0x0043CAB8`; art INI | partially matched by `refinery_can_dock_queue_cell`; chrono staging still uses QueueingCell | `src/sim/miner/miner_dock_sequence.rs:100`, `src/sim/miner/miner_system.rs:1030` | Keep accepted-cell movement separate from wait/staging cell until runtime proves staging parity | Miner at `(13,11)` for refinery `(10,10)` can trigger dock handshake | Do not collapse QueueingCell and CAN_DOCK accepted cell |
| Exact same-frame second-miner takeover is not statically proven | deferred watchpoints | unchecked | `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs` | Add tests only after runtime trace pins frame/tick boundary | Runtime trace determines whether handoff is same-frame, next-frame, or later retry | Do not hardcode a guessed one-tick delay as parity |

### Stale Docs / Follow-up Docs

- `miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md` Stage 8 should be read as superseded. Replacement wording: "Stock normal handoff is tied to zero-link `Mission_Deploy_Building` state-4 `BREAK(0x03)` contact cleanup after `+0x57C` clears, not to `ReleaseDockedHarvester` and not to arrival at a Rust exit target."
- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` remains useful but YELLOW. Use the 2026-05-21 correction banner and this report's ordering for stock state-4 queue handoff.
- `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` is RED in the audit log for branch wording; use the later branch/writer reports and this report for state-4 handoff claims.

## Sources

- Ghidra decompiled/read-only: `0x0073D630`, `0x0043C2D0`, `0x0065A820`, `0x0065A970`, `0x0065ACB0`, `0x0065AD90`, `0x0065ADF0`, `0x0065AE30`, `0x0065AE60`, `0x006F4AB0`, `0x004D9290`, `0x00741970`, `0x00739EC0`, `0x00737430`.
- Ghidra assembly contexts/read-only: `0x0073E1D5`, `0x0073E1DF`, `0x0073E1F6`, `0x0073E24F`, `0x0073E26A`, `0x0073E275`, `0x0073E27F`, `0x0043C8A4`, `0x0043C8B3`, `0x0043C9F5`, `0x0043CAB4`, `0x0043CACA`, `0x0043CADB`, `0x0065A9C9`, `0x0065AA2E`, `0x0065AA4F`, `0x0073A4FB`, `0x0073A503`.
- Prior docs: `miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md`, `miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `AUDIT_LOG.md`, `miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `miner/MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`, `miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`.
