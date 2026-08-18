# Chrono Miner Refinery Contact Saturation / Queue Eviction - Ghidra Research Report

**Address(es):** `0x0065A970`, `0x0065A820`, `0x0065ADF0`, `0x0043C2D0`, `0x004D9290`, `0x004D8FB0`, `0x0073E5E0`, `0x0041AA80`, `0x00500200`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR `CMIN`/`HARV` contention for stock `GAREFN`/`NAREFN` radio contacts: HELLO full-slot behavior, sender-side eviction, receiver-side full-slot behavior, `BuildingClass::Receive_Radio` `0x0E` and `0x08`, `0x17` queued replies, and when `QueueingCell=4,1` is used after non-acceptance.  
**Non-Scope:** exact rendered frame timing of the next miner's takeover after state-4 unload exit, save/load persistence of contacts, non-stock multi-dock refinery mods, full factory/repair/bunker queue systems, and slave miner/Yuri refinery behavior.  
**Confidence:** High for static branch behavior and stock gates; Medium for exact same-frame multi-miner player-visible ordering because that still needs runtime trace confirmation.  
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN`; conditional branches are called out per finding.

## 1. Overview

Stock refinery contention is not a separate building queue opcode. The live path is a mix of `RadioClass` contact capacity, `Mission_Harvest` fallback staging, repeated `Mission_Enter` `CAN_DOCK(0x0E)`, and physical accepted-cell arrival.

The important split is: `HELLO(0x02)` can reject when the refinery's `Contacts[]` slot is full, but `BuildingClass::Receive_Radio(0x0E)` does not hard-return `NEGATORY(10)` just because the contact slot is full. The receiver can still send `0x13`/`0x12` and return `ROGER(1)` without sending the final `0x18/0x16` enter burst.

## 2. Class Layout / Key Offsets

| Owner | Offset / field | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `RadioClass` | `+0xE4` | `Contacts[]` pointer | Yes |
| `RadioClass` | `+0xE8` | contact capacity, not live count | Yes |
| `RadioClass` | `+0xD4/+0xD8/+0xDC` | 3-deep radio history push-down | Yes, not a handler skip |
| `ObjectClass` | `+0x6C` | HELLO alive/in-map guard in base receiver | Yes; exact field name out-of-scope |
| `AbstractClass` | `+0x14 bit 0` | reverse ally-check gate for HELLO | Yes when flag set |
| `BuildingTypeClass` | `+0x16A9` | `UnitRepair=yes`; special queue/repair branch | Conditional; false for stock GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x16AB` | `Bunker=yes`; special queue/link branch | Conditional; false for stock GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x16B3` | `DockUnload=yes`; stock refinery handoff branch | Yes for GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x16BB` | `Refinery=yes`; refinery context | Yes for GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x16BD` | `WeaponsFactory=yes`; enables `0x08 -> 0x17` | No for stock GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x16C1/+0x16C2` | `Hospital/Armory`; alternate `0x0E` queue/evict logic | No for stock GAREFN/NAREFN |
| `BuildingTypeClass` | `+0x1618/+0x161C` | `QueueingCell` X/Y from art | Yes in `Mission_Harvest` fallback, not in building `0x0E` |
| `FootClass` | `+0x5A4` (`param_1[0x169]`) | current NavCom/destination pointer | Yes |
| `FootClass` | `+0xB4` (`param_1[0x2D]`) | current/queued mission comparison field in radio cases | Yes |
| `UnitClass` | `+0x6D1` | unload-active latch, state-4 exit cleanup | Yes, but exit timing out-of-scope here |

## 3. Core Logic

### 3.1 Sender-side HELLO eviction

`RadioClass::Transmit_Radio_Impl @ 0x0065A970` handles outgoing `HELLO(0x02)` specially:

1. If the explicit target is already in the sender's `Contacts[]`, return `ROGER(1)` immediately and do not call the receiver.
2. If a free sender contact slot exists, remember its index.
3. If no free sender slot exists, send `BREAK(0x03)` to `Contacts[0]` through vtable slot `+0x278`, then reuse slot `0`.
4. Dispatch `HELLO` to the target receiver.
5. Only if the receiver returns `ROGER(1)`, write the target into the sender's freed slot; otherwise return `NEGATORY(10)`.

Tiny details:

- The eviction is on the sender's contact vector, not the receiver's.
- The eviction uses `Transmit_Radio`, not `Transmit_Radio_Impl`, so subclass overrides would participate.
- A sender whose old contact is evicted does not add the new contact unless the receiver accepts the new HELLO.
- For a stock harvester sender, contact capacity is normally `1`, so changing refinery contact can evict the old contact. This is active YR behavior.

### 3.2 Receiver-side HELLO full-slot behavior

`RadioClass::Receive_Radio @ 0x0065A820` handles incoming `HELLO(0x02)`:

1. Shift radio history if the message differs from the most recent history entry.
2. Require receiver `+0x6C != 0`; otherwise fall through to `ObjectClass::Receive_Radio`.
3. Run receiver-perspective ally check.
4. If `AbstractClass+0x14 bit 0` is set, run reverse ally check.
5. If sender is already in any contact slot, return `ROGER(1)`.
6. Otherwise scan for a null slot; fill the first null slot and return `ROGER(1)`.
7. If no null slot exists, return `NEGATORY(10)`.

Tiny details:

- Receiver HELLO does not evict. Full receiver slots return `10`.
- The already-linked scan and free-slot scan are two separate passes.
- `Contacts.Capacity < 1` returns `10`.
- BREAK receiver handling clears only the first matching slot; BREAK sender handling clears all matching slots before dispatch.

### 3.3 Contact-free helper used by building receiver

`FUN_0065ADF0 @ 0x0065ADF0` scans `Contacts[]` and returns low byte true only if a slot is null or already equals the candidate target.

For a stock one-dock refinery:

- If no miner is contacted, helper returns true.
- If the same miner is already contacted, helper returns true.
- If another miner occupies the single slot, helper returns false.

This helper prevents `BuildingClass::Receive_Radio(0x0E)` from initiating a building-side HELLO when the refinery is already full, but it does not by itself hard-reject the rest of the `0x0E` branch.

### 3.4 `BuildingClass::Receive_Radio(0x0E)` with full contacts

`BuildingClass::Receive_Radio @ 0x0043C2D0` first calls `TechnoClass::Receive_Radio`, then applies stock receiver gates.

Stock DockUnload branch facts:

- `HasPower=false` returns `10`. Active in YR when power is down.
- `UnitRepair` and `Bunker` reject gates are false for stock GAREFN/NAREFN.
- `Hospital/Armory` branch is not stock refinery.
- If the sender is not in contacts and `FUN_0065ADF0` returns true, the building sends HELLO to the sender and re-checks contact membership.
- If `FUN_0065ADF0` returns false because the one stock slot is full with another miner, the standard DockUnload branch still continues to `0x13` and `0x12`.
- The hardcoded `0x12` target is building packed/NW cell `+(3,1)`.
- `QueueingCell=4,1` is not read in this function.
- If `FootClass::Receive_Radio(0x12)` returns anything other than `0x14`, the building returns `1` and does not send `0x18` or `0x16`.
- Only `0x12 == 0x14` triggers `0x18 ENTER_DOCK` then `0x16 TIMING_SYNC`.

Static consequence for saturation: no-free-contact does not equal `0x0E` hard rejection. The more important observable blocker is whether the requesting miner can occupy/reach the hardcoded accepted cell and whether the previous miner still owns the physical/contact-entered state.

### 3.5 `FootClass::Receive_Radio(0x12/0x13/0x17)`

`FootClass::Receive_Radio @ 0x004D8FB0` confirms the unit-side replies:

- `0x13 NEED_TO_MOVE`: writes current destination (`+0x5A4`) to payload. If a locomotor/helper says the unit is currently moving, returns `10`; otherwise returns `1`.
- `0x12 MOVE_TO_CELL`: if the payload cell matches the unit's current packed cell, returns `0x14`. Otherwise it sets destination through vtable `+0x480`, writes current frame into `+0xC8`, clears `+0xD0`, and returns `1`.
- `0x17 QUEUED`: if the current path is valid and current destination equals `+0x5A4`, clear destination; if current mission is idle or enter, queue mission `5`; may play a sound if no destination and a flag is clear.

`0x17` is therefore a real unit-side reply handler, but stock refineries do not normally send it from their `0x08` path.

### 3.6 `BuildingClass::Receive_Radio(0x08)` and `0x17`

For `case 0x08`, `BuildingClass::Receive_Radio @ 0x0043C2D0` does:

1. If `UnitRepair` or `Bunker`, and sender is within `0x180` leptons, return `1`.
2. Call `TechnoClass::Receive_Radio(0x08)`.
3. If not `WeaponsFactory`, not `UnitRepair`, and not `Bunker`, return `1`.
4. Otherwise return `0x17`.

Stock GAREFN/NAREFN have `DockUnload=yes` and `Refinery=yes`; they do not have `WeaponsFactory`, `UnitRepair`, or `Bunker`. Therefore stock refinery `0x08` returns `1`, not `0x17`. `0x17` queued replies are active YR behavior for factory/repair/bunker-style paths, not for stock ore refinery queue admission.

### 3.7 `Mission_Harvest` fallback and `QueueingCell=4,1`

`UnitClass::Mission_Harvest @ 0x0073E5E0` is where stock `QueueingCell` is verified in this slice.

In return state 2:

- The unit finds a refinery via vtable `+0x528`.
- If the miner/refinery distance is within the relevant threshold, it sends `HELLO(0x02)`.
- Standard HARV uses `RulesClass+0xD78` (`HarvesterTooFarDistance=5` cells in `rulesmd.ini`).
- CMIN uses `RulesClass+0xD7C` (`ChronoHarvTooFarDistance=50` cells in `rulesmd.ini`).
- If HELLO returns `1`, it writes harvest substate `3`; state 3 queues mission `7` (`Mission_Enter`).
- If the close HELLO cannot proceed, or the chosen refinery is too far, fallback code finds a docking target again with the fog/map-editor bypass active, then checks `distance > 0x300 || is_chrono`.
- On that fallback, it reads `BuildingTypeClass+0x1618/+0x161C`, adds those shorts to the refinery packed/NW cell, and calls `FootClass::Find_Nearby_Passable_Cell` with radius `2`.
- If no passable cell is found, it clears destination; otherwise it sets destination to the found cell.

Answer to the targeted QueueingCell question:

- Building `0x0E` non-accepted/not-arrived does not use `QueueingCell`.
- Close-return HELLO non-acceptance can flow into `Mission_Harvest` fallback.
- For CMIN, the fallback condition is always true because `is_chrono` is true, so stock `QueueingCell=4,1` is used as the staging seed after close HELLO cannot proceed.
- For normal HARV, the fallback uses `QueueingCell` when the fallback target is farther than `0x300` leptons (3 cells) under the same state-2 fallback path.

### 3.8 `Mission_Enter` caller behavior

`FootClass::Mission_Enter @ 0x004D9290` sends `CAN_DOCK(0x0E)` to the current destination. If the return is `1`, it continues mission-enter/path handling. If the return is not `1` and the contact-entered byte (`+0x418`) is false, it sends `BREAK(0x03)` to first contact and clears destination.

This means normal `0x0E` deferrals that return `1` keep the unit in the retry/approach path; they are not treated as a hard rejection.

### 3.9 Unit-side enter/dock helper

`UnitClass::EnterBuildingOrDock @ 0x0041AA80` confirms the same contact helper is used before some enter/dock actions:

- If the target is a building and the unit is in mission-enter context, it checks path validity and `FUN_0065ADF0`.
- If no free/same contact exists in the unit-side contact vector, it may set the building as a ghost cell / destination rather than completing a contact handshake.
- If contact exists and `DynamicVectorClass::Contains` is false, it can send `0x0E`; if the reply is not `1`, it sends `BREAK(0x03)`.

This function is active for building/dock interactions, but the exact split between player command entry and autonomous harvester return is broader than this slice.

### 3.10 `FUN_00500200` nearby passable cell helper

`FUN_00500200 @ 0x00500200` is a generic nearby-passable-cell picker:

- It samples one of several object-side coordinates based on random `1..4` when any of three object methods return nonzero.
- It uses the unit's current packed cell as a zone seed.
- It calls `FootClass::Find_Nearby_Passable_Cell` with radius `1`.

No read of `QueueingCell` appears here. In this slice, the verified `QueueingCell` use is in `Mission_Harvest @ 0x0073E5E0`, not this helper.

## 4. INI Keys

| INI key | Stock value | Evidence | Binary effect | Active in YR |
|---|---|---|---|---|
| `[General] HarvesterTooFarDistance` | `5` | `ini/rulesmd.ini:293` | Standard harvester close HELLO threshold in state 2 | Yes |
| `[General] ChronoHarvTooFarDistance` | `50` | `ini/rulesmd.ini:294` | Chrono miner close HELLO threshold in state 2 | Yes |
| `[CMIN] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | Candidate refinery types | Yes |
| `[CMIN] Harvester` | `yes` | `ini/rulesmd.ini:7364` | Enables harvester mission family | Yes |
| `[CMIN] Teleporter` | `yes` | `ini/rulesmd.ini:7396` | Selects chrono threshold/fallback behavior | Yes |
| `[CMIN] Storage` | `20` | `ini/rulesmd.ini:7374` | Cargo capacity context | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:8225` | Candidate refinery types | Yes |
| `[HARV] Harvester` | `yes` | `ini/rulesmd.ini:8228` | Enables harvester mission family | Yes |
| `[HARV] Storage` | `40` | `ini/rulesmd.ini:8236` | Cargo capacity context | Yes |
| `[GAREFN] DockUnload` | `yes` | `ini/rulesmd.ini:11726` | Building `0x0E`/`0x15` stock branch | Yes |
| `[GAREFN] Refinery` | `yes` | `ini/rulesmd.ini:11727` | Refinery context | Yes |
| `[GAREFN] NumberOfDocks` | `1` | `ini/rulesmd.ini:11729` | Radio contact capacity for building | Yes |
| `[NAREFN] DockUnload` | `yes` | `ini/rulesmd.ini:12519` | Building `0x0E`/`0x15` stock branch | Yes |
| `[NAREFN] Refinery` | `yes` | `ini/rulesmd.ini:12520` | Refinery context | Yes |
| `[NAREFN] NumberOfDocks` | `1` | `ini/rulesmd.ini:12521` | Radio contact capacity for building | Yes |
| `[GAREFN]/[NAREFN] QueueingCell` | `4,1` | `ini/artmd.ini:1773`, `1716` | `Mission_Harvest` fallback seed, not building `0x0E` target | Yes, conditional |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | sender HELLO eviction and direct dispatch | fresh decompile; assembly `0x0065AA2A..0x0065AA36` | Yes |
| `RadioClass::Receive_Radio @ 0x0065A820` | receiver HELLO/BREAK contact list | fresh decompile; assembly `0x0065A8D8..0x0065A8FA` | Yes |
| `FUN_0065ADF0 @ 0x0065ADF0` | free/matching contact probe | fresh decompile | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | stock refinery `0x0E`, `0x08`, `0x15` receiver | fresh decompile; assembly `0x0043C788..0x0043C7A0` | Yes / Conditional |
| `FootClass::Receive_Radio @ 0x004D8FB0` | unit replies `0x12`, `0x13`, `0x17` | fresh decompile; assembly `0x004D9180..0x004D9193` | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | sends `0x0E`, handles return code | fresh decompile | Yes |
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | close HELLO, fallback QueueingCell staging | fresh decompile | Yes |
| `UnitClass::EnterBuildingOrDock @ 0x0041AA80` | unit-side dock/contact helper | fresh decompile | Yes |
| `FUN_00500200 @ 0x00500200` | nearby passable helper, no QueueingCell read | fresh decompile; assembly `0x00500200..` | Conditional |

## 6. Current Rust Implementation Status

Rust has a clean abstraction in `src/sim/miner/miner_dock.rs`:

- `RefineryDockContacts::hello_or_wait` accepts up to `capacity`, queues rejected miners FIFO, and never evicts the current contact.
- This matches receiver-side full-slot behavior, but it does not model sender-side HELLO eviction of the sender's old contact.
- Stock refinery capacity is pulled from `NumberOfDocks` through `resolve_refinery_cells` in `src/sim/miner/miner_dock_sequence.rs`.

Rust splits the two important cells:

- `refinery_can_dock_queue_cell(rx, ry)` returns `(rx+3, ry+1)` for building `0x0E`.
- `refinery_queue_cell(...)` uses art `QueueingCell=4,1` for waiting/far-return staging.

That split matches the binary distinction. The remaining mismatch risk is exact behavior when a second miner is denied close HELLO: gamemd's `Mission_Harvest` fallback runs a radius-2 nearby-passable search around `QueueingCell`; Rust's current dock approach queue movement and chrono return staging are similar in shape but not proven same-frame equivalent.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Sender-side HELLO full-slot eviction | verified | `0x0065A970`; assembly `0x0065AA2A..0x0065AA36` | none for static behavior |
| Receiver-side HELLO full-slot reject | verified | `0x0065A820` | exact `+0x6C` field name |
| `FUN_0065ADF0` free/same probe | verified | `0x0065ADF0` | none |
| Building `0x0E` stock DockUnload full-contact path | verified | `0x0043C2D0` | live two-miner frame ordering |
| Building `0x0E` `QueueingCell` non-use | verified | `0x0043C2D0` | none |
| Building `0x08` stock refinery return | verified | `0x0043C2D0`; stock INI | none |
| `0x17` queued replies | verified conditional | `0x0043C2D0`; `0x004D8FB0` | full factory/repair/bunker queue system out-of-scope |
| `Mission_Harvest` close HELLO and QueueingCell fallback | verified | `0x0073E5E0`; stock INI | exact same-frame path after HELLO rejection needs trace |
| `FootClass::Mission_Enter` return handling | verified | `0x004D9290` | full mission scheduler frame timing deferred |
| `UnitClass::EnterBuildingOrDock` contact helper | touched-not-exhausted | `0x0041AA80` | player-command vs autonomous miner split |
| `FUN_00500200` queue/passable helper | verified negative for QueueingCell | `0x00500200` | exact callers beyond prior docs |
| Rust implementation scan | verified structurally | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs` | focused parity tests still needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Is this stock YR-active? -> Yes; CMIN/HARV dock to NAREFN/GAREFN, stock refineries are DockUnload/Refinery with NumberOfDocks=1.` (evidence: `ini/rulesmd.ini:7361`, `8225`, `11726-11729`, `12519-12521`)
- `[RESOLVED] OQ-002 - Does sender-side HELLO evict on full contacts? -> Yes; sender sends BREAK to Contacts[0], reuses slot 0 only if the new receiver returns ROGER.` (evidence: `0x0065A970`)
- `[RESOLVED] OQ-003 - Does receiver-side HELLO evict on full contacts? -> No; receiver returns NEGATORY when all slots are full and sender is not already present.` (evidence: `0x0065A820`)
- `[RESOLVED] OQ-004 - What does the building contact-free helper test? -> Empty slot or same sender only.` (evidence: `0x0065ADF0`)
- `[RESOLVED] OQ-005 - Does stock building 0x0E hard-reject solely because the contact slot is full? -> No; the standard DockUnload branch can continue to 0x13/0x12 and return 1.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-006 - Does building 0x0E use QueueingCell=4,1? -> No; it computes anchor+(3,1).` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-007 - Does building 0x08 return 0x17 for stock refineries? -> No; stock GAREFN/NAREFN lack WeaponsFactory, UnitRepair, and Bunker, so return 1.` (evidence: `0x0043C2D0`; stock INI)
- `[RESOLVED] OQ-008 - Is 0x17 real? -> Yes, as a unit-side handler and conditional building reply for factory/repair/bunker paths, not stock refinery queue admission.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-009 - When is QueueingCell used after non-acceptance? -> In Mission_Harvest state-2 fallback after close HELLO cannot proceed or target is too far; CMIN always passes the fallback distance clause because is_chrono is true.` (evidence: `0x0073E5E0`; `ini/artmd.ini:1716`, `1773`)
- `[RESOLVED] OQ-010 - Does FUN_00500200 read QueueingCell? -> No; it picks a nearby passable cell around a helper-selected coordinate and unit zone seed.` (evidence: `0x00500200`)
- `[RESOLVED] OQ-011 - What does 0x12 do when the unit is not already at the cell? -> Sets destination/timing fields and returns 1; no 0x18/0x16 from building unless 0x12 returned 0x14.` (evidence: `0x004D8FB0`, `0x0043C2D0`)
- `[RESOLVED] OQ-012 - What does Mission_Enter do with 0x0E return 1? -> Treats it as continue/retry, not a hard rejection.` (evidence: `0x004D9290`)
- `[DEFERRED] OQ-013 - Exact frame when the next miner takes contact after the current miner exits state 4.` (category: `needs-runtime-debugger`; reason: static code proves branch shape but not replay frame ordering; next-step-if-pursued: run the requested trace-action for full-cargo close return dispatch timing)
- `[DEFERRED] OQ-014 - Can a contrived already-at-accepted-cell second miner receive 0x18/0x16 while refinery Contacts[] is full?` (category: `needs-runtime-debugger`; reason: static 0x0E lacks a contact hard gate after 0x12==0x14, but physical occupancy and mission timing decide if this happens in normal play; next-step-if-pursued: trace two miners forced around `(rx+3,ry+1)`)
- `[DEFERRED] OQ-015 - Full factory/repair/bunker 0x17 queue semantics.` (category: `out-of-scope`; reason: stock refinery branch is resolved; next-step-if-pursued: separate repair/factory docking investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Incoming HELLO to a full stock refinery returns NEGATORY and does not evict the current refinery contact. | `0x0065A820`; `NumberOfDocks=1` INI | mostly matched | `src/sim/miner/miner_dock.rs::RefineryDockContacts::hello_or_wait` | Keep accepted contact stable while rejected miners wait/retry. | Two full CMIN/HARV target one refinery; first contact remains owner until release. | Do not replace current contact just because another miner says HELLO. |
| Outgoing HELLO from a full sender contact vector evicts the sender's old `Contacts[0]` via BREAK before trying the new target. | `0x0065A970` | missing/unchecked | future generic radio/contact abstraction, or miner target-change cleanup | If modeling generic radio, sender target switches must break old contact before adding new only on receiver ROGER. | Miner retargets from refinery A to B while still radio-linked; old contact is broken, new link exists only if B accepts. | Do not implement eviction on the receiver side. |
| Stock building `0x0E` does not hard-reject solely from full Contacts[]; it can return `1` after `0x13`/`0x12` without `0x18/0x16`. | `0x0043C2D0` | partial abstraction risk | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` | Distinguish retry/approach from final dock-enter. | Waiting miner repeatedly approaches accepted cell but cannot enter/unload while first miner still owns contact/pad. | Do not treat every busy `0x0E` as immediate `NEGATORY`. |
| Final enter burst requires `0x12 == 0x14` at hardcoded anchor `+(3,1)`. | `0x0043C2D0`; `0x004D8FB0` | matched for cell split | `refinery_can_dock_queue_cell`, `phase_mission_enter` | Keep accepted-cell target separate from art `QueueingCell`. | GAREFN `(10,10)` accepted target is `(13,11)`; miner only links once there and not moving. | Do not use `QueueingCell=4,1` as the `0x0E` accepted cell. |
| Stock refinery `0x08` returns `1`; `0x17` queued is factory/repair/bunker conditional, not stock refinery admission. | `0x0043C2D0`; `0x004D8FB0`; stock INI | matched in broad shape | any future radio opcode model | Do not add a stock refinery `0x08 -> 0x17` queue state. | Sending `0x08` to GAREFN/NAREFN does not move miner into queued mission via 0x17. | Do not use 0x17 as normal ore refinery busy reply. |
| After close HELLO non-acceptance, CMIN state-2 fallback can use `QueueingCell=4,1` with radius-2 passable search. | `0x0073E5E0`; `ini/artmd.ini` | partial; Rust has queue/far staging but exact passable radius/timing should be tested | `src/sim/miner/miner_system.rs::chrono_return_staging_cell_for_sid`; `phase_approach` wait movement | Use `QueueingCell` as staging/fallback seed, not as final dock cell; include nearby-passable fallback. | Two CMIN same refinery, second rejected by HELLO, expected staging around `(rx+4,ry+1)` if passable. | Do not remove QueueingCell entirely from waiting/far-return behavior. |
| `FUN_00500200` does not consume QueueingCell. | `0x00500200` | none observed | none unless modeling that helper | Keep it separate from refinery art queue behavior. | No Rust acceptance needed for this slice. | Do not cite `FUN_00500200` as proof of QueueingCell use. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md`: replace any claim that stock `0x0E` returns `NEGATORY` solely because the refinery contact slot is full with: "Receiver HELLO can reject when full, but stock DockUnload `0x0E` can still return `ROGER(1)` after `0x13`/`0x12`; no final `0x18/0x16` enter burst occurs unless `0x12` returns `0x14`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`: refine `0x08` wording to state that `0x17` is conditional for `WeaponsFactory/UnitRepair/Bunker`, not stock GAREFN/NAREFN.
- `docs/gap-scans/2026-05-19-disparity-scan-miner.md`: update G10 to distinguish the fixed accepted `CAN_DOCK` target `(rx+3,ry+1)` from valid `QueueingCell=4,1` fallback/staging uses after HELLO non-acceptance or far return.
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/structures/GAREFN.md` and `NAREFN.md`: keep `QueueingCell=4,1` as a valid fallback/waiting cell, but remove or qualify any implication that it is the building `0x0E` accepted cell.

## 10. Negative Facts / Do Not Do

- Do not implement receiver-side HELLO eviction. Receiver full-slot behavior is `NEGATORY(10)`.
- Do not treat sender-side HELLO eviction as stock refinery queue promotion; it is the sender clearing its own old contact before attempting a new contact.
- Do not model stock GAREFN/NAREFN `0x08` as returning `0x17`.
- Do not use `QueueingCell=4,1` for building `0x0E` accepted/not-arrived `0x12` payload.
- Do not remove `QueueingCell=4,1` from all miner behavior; `Mission_Harvest` fallback uses it after close HELLO cannot proceed or when the fallback condition is met.
- Do not conflate `Contacts[]` saturation with physical pad occupancy. Static code has separate contact and movement/accepted-cell gates.

## 11. Sources

- Fresh Ghidra decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`
- Fresh Ghidra decompile: `RadioClass::Receive_Radio @ 0x0065A820`
- Fresh Ghidra decompile: `RadioClass::Transmit_Radio @ 0x0065AAA0`
- Fresh Ghidra decompile: `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0`
- Fresh Ghidra decompile: `RadioClass::Set_Contact_Count @ 0x0065AE60`
- Fresh Ghidra decompile: `FUN_0065ADF0 @ 0x0065ADF0`
- Fresh Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Fresh Ghidra decompile: `FootClass::Mission_Enter @ 0x004D9290`
- Fresh Ghidra decompile: `FootClass::Receive_Radio @ 0x004D8FB0`
- Fresh Ghidra decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`
- Fresh Ghidra decompile: `UnitClass::EnterBuildingOrDock @ 0x0041AA80`
- Fresh Ghidra decompile: `FUN_00500200 @ 0x00500200`
- Fresh Ghidra assembly contexts: `0x0065AA2A..0x0065AA36`, `0x0065A8D8..0x0065A8FA`, `0x0043C788..0x0043C7A0`, `0x004D9180..0x004D9193`, `0x00500200..`
- Prior synthesis: `C:/Users/enok/Documents/ra2-rust-game-docs/CHRONO_MINER_NAVCOM_RADIO_SYSTEM_MODEL_SYNTHESIS.md`
- Prior focused reports: `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_REFINERY_0X0E_NON_ACCEPTED_PATHS_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE_HANDOFF_EXIT_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust scanned: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`
