# Building Receive Radio Refinery 0x0E Non-Accepted Paths - Ghidra Research Report

**Address(es):** `BuildingClass::Receive_Radio @ 0x0043C2D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** refinery / DockUnload `CAN_DOCK(0x0E)` paths in `BuildingClass::Receive_Radio` that do not complete the accepted `0x18`/`0x16` dock-enter burst.  
**Non-Scope:** accepted anchor math beyond contrast, post-arrival `0x15`, unload FSM, normal exit/release, war-factory production exits, and full unit-side queue path.  
**Confidence:** High for receiver-branch behavior; Medium for Rust delta where current implementation was scanned structurally only.  
**Active in YR:** Yes. Standard `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`; `[CMIN]` and `[HARV]` have `Dock=NAREFN,GAREFN` and `Harvester=yes` in `rulesmd.ini`.

## 1. Overview

`CAN_DOCK(0x0E)` is a synchronous receiver-side admission call. On standard DockUnload refineries, the building can hard-reject only a few gates (`HasPower=false`, UnitRepair/Bunker-special gates); otherwise even non-final states usually return `ROGER(1)` while the unit is told to move, wait, or retry.

The important non-accepted outcome is: if `MOVE_TO_CELL(0x12)` returns `1` instead of `0x14`, the building returns `1` and does not send `ENTER_DOCK(0x18)` or `TIMING_SYNC(0x16)`. That means the caller should keep trying through `Mission_Enter`, not treat the dock as occupied by the requester.

## 2. Class Layout / Key Offsets

| Field / flag | Offset | Purpose in this slice | Active in YR |
|---|---:|---|---|
| `Contacts[]` pointer | `RadioClass+0xE4` | Contact roster used by Contains/free-slot checks | Yes; `NumberOfDocks=1` on stock refineries |
| `Contacts` capacity | `RadioClass+0xE8` | Scanned by `FUN_0065ADF0` | Yes |
| `BuildingClass::HasPower` | decompiler field | If false, `0x0E` returns `10` | Conditional; stock power state can change |
| `UnitRepair=` | `BuildingType+0x16A9` | Special busy/repair gate, not stock refinery | No for GAREFN/NAREFN |
| `Bunker=` | `BuildingType+0x16AB` | Special auto-deploy gate, not stock refinery | No for GAREFN/NAREFN |
| `DockUnload=` | `BuildingType+0x16B3` | Standard refinery branch | Yes for GAREFN/NAREFN |
| `Weeder=` | `BuildingType+0x16BC` | Same branch family, TS/weed path | No for stock YR refinery |
| `Hospital=` / `Armory=` | `+0x16C1` / `+0x16C2` | Alternate queue/repair branch, not stock refinery | No for GAREFN/NAREFN |
| `Helipad=` | `+0x16CB` | Alternate non-DockUnload path | No for GAREFN/NAREFN |
| `QueueingCell=` storage | `BuildingType+0x1618/+0x161C` per prior docs | Not read anywhere in `0x0043C2D0` case `0x0E` | Parsed for stock art but inactive in this receiver |

## 3. Core Logic - Non-Accepted Outcomes

For `case 0x0E`, `BuildingClass::Receive_Radio` first delegates to `TechnoClass::Receive_Radio`, then applies receiver gates.

1. **Hard no-power reject:** if `HasPower == false`, returns `10`.  
   Active in YR: Conditional. Evidence: live decompile `0x0043C2D0` case `0x0E`; stock refineries use this live building field.

2. **Special UnitRepair/Bunker rejects:** if `UnitRepair` contact is present and `0x22` returns `10`, return `10`; if `Bunker` contact is present and `CanAutoDeployHere(sender)` fails, return `10`.  
   Active in YR: No for stock GAREFN/NAREFN; the code is live for other building types. Evidence: `0x0043C2D0`, flags `+0x16A9/+0x16AB`; `rulesmd.ini` stock GAREFN/NAREFN do not set these flags.

3. **Contact/free-slot probe:** for non-Hospital/Armory buildings, the code checks whether the sender is already in `Contacts[]`. If not present and `FUN_0065ADF0` finds either an empty slot or the same sender, the building transmits `HELLO(0x02)` to the sender and re-checks `Contains`. `FUN_0065ADF0 @ 0x0065ADF0` returns low byte `1` only for a zero slot or matching sender; otherwise low byte `0`.  
   Active in YR: Yes. Evidence: decompile `0x0043C2D0`; helper decompile `0x0065ADF0`; `NumberOfDocks=1` for GAREFN/NAREFN.

4. **No free contact slot is not a hard `0x0E` reject by itself:** if the sender is not in contacts and no free slot is found, this standard DockUnload branch still proceeds to send `NEED_TO_MOVE(0x13)` and then `MOVE_TO_CELL(0x12)` directly to `param_2`; there is no immediate `return 10` in this branch.  
   Active in YR: Yes for the receiver function. Evidence: `0x0043C2D0` standard branch continues past the contact/free-slot block; `RadioClass::Transmit_Radio_Impl @ 0x0065A970` directly calls the explicit target receiver for non-HELLO messages.

5. **Need-to-move early return:** the building sends `0x13` before assigning a cell. `FootClass::Receive_Radio @ 0x004D8FB0` case `0x13` writes the unit's `+0x5A4` field into payload, and if that field is nonzero and the locomotor reports moving, returns `10`. If the building sees a non-`1` reply and the decompiler's local sentinel low byte is zero, it returns `1` immediately. It does not send `0x12`, `0x18`, or `0x16`.  
   Active in YR: Conditional; relevant to chrono/piggyback movement state. Evidence: `0x0043C2D0` `0x13` send/return branch; `0x004D8FB0` case `0x13`; CMIN has `Teleporter=yes`.

6. **Move-but-not-arrived path:** standard DockUnload computes the same hardcoded cell as the accepted path, writes the `CellClass*` to `*param_4`, sends `MOVE_TO_CELL(0x12)`, and if the unit returns anything other than `0x14`, returns `1`. It does not send `0x18` or `0x16`.  
   Active in YR: Yes. Evidence: `0x0043C2D0`; `FootClass::Receive_Radio @ 0x004D8FB0` case `0x12` returns `1` after `SetDestination(payload, 1)` when not already at the target cell.

7. **Already-at-cell is the acceptance gate:** only when `0x12` returns `0x14` does the building send `0x18` and `0x16`. This report treats that branch only as contrast.  
   Active in YR: Yes. Evidence: `0x0043C2D0` standard DockUnload branch; `0x004D8FB0` `0x12` already-there check.

8. **`QueueingCell=4,1` is not consumed in `0x0043C2D0` case `0x0E`:** the function reads the building's packed map cell through vtable `+0x1B8`, adds `(3,1)`, calls `MapClass__Get_CellClass`, and passes that cell to `0x12`. No read of `BuildingType+0x1618/+0x161C` appears in this receiver path.  
   Active in YR: Yes. Evidence: live decompile `0x0043C2D0`; `artmd.ini` stock `QueueingCell=4,1` exists but is not referenced here.

## 4. INI Keys

| Key | Stock YR values | Effect in this slice | Active in YR |
|---|---|---|---|
| `[GAREFN]/[NAREFN] DockUnload=yes` | yes | Enters standard DockUnload branch | Yes |
| `[GAREFN]/[NAREFN] Refinery=yes` | yes | Context flag, not directly the `0x0E` DockUnload branch gate | Yes elsewhere |
| `[GAREFN]/[NAREFN] NumberOfDocks=1` | yes | Contact capacity used by RadioClass | Yes |
| `[GAREFN]/[NAREFN] QueueingCell=4,1` | artmd yes | Not read by this receiver branch | No in this function |
| `[CMIN]/[HARV] Dock=NAREFN,GAREFN` | yes | Unit-side selection of receiver | Yes |
| `[CMIN]/[HARV] Harvester=yes` | yes | Unit-side path context | Yes |
| `[CMIN] Teleporter=yes` | yes | Makes `0x13` chrono/moving branch relevant | Conditional |

## 5. Integration Points

`FootClass::Mission_Enter @ 0x004D9290` sends `CAN_DOCK(0x0E)` to its current destination. If the result is `1`, it continues mission-enter handling; if result is not `1` and the unit's `+0x418` byte is false, it sends `BREAK(0x03)` to first contact and clears destination.

`RadioClass::Transmit_Radio @ 0x0065AAA0` is declared `void` by Ghidra but passes through the result from `Transmit_Radio_Impl @ 0x0065A970`. For non-HELLO/non-BREAK messages, `Transmit_Radio_Impl` directly invokes the explicit target receiver; the target does not need to be in the sender's contact list.

## 6. Current Rust Implementation Status

The current Rust miner code already has a separate hardcoded `refinery_can_dock_queue_cell(rx, ry) -> (rx+3, ry+1)` in `src/sim/miner/miner_dock_sequence.rs:104`, matching the building receiver's `0x12` target for this radio path.

The broader reservation model is not wire-equivalent. `src/sim/miner/miner_dock.rs:219` `DockReservations::try_reserve` enqueues and returns `false` when occupied, and `src/sim/miner/miner_dock_sequence.rs:588-593` marks `dock_queued=true` and moves toward `wait_queue`, which comes from `refinery_queue_cell` and therefore uses art `QueueingCell=4,1`. That may be a useful gameplay abstraction, but it should not be described as the exact receiver-side `0x0E` non-accepted behavior.

Current Rust far-return staging also uses `refinery_queue_cell` in `src/sim/miner/miner_system.rs:1042`, which is outside this receiver slice and remains a separate unit-side/staging question.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` standard DockUnload branch | verified | live decompile `0x0043C2D0` | none for scoped receiver |
| Hard reject: `HasPower=false` | verified | `0x0043C2D0`; `rulesmd.ini` active refinery path | runtime power timing outside scope |
| UnitRepair/Bunker special rejects | verified | `0x0043C2D0`; flags `+0x16A9/+0x16AB` | not stock refinery |
| Contact/free-slot probe | verified | `0x0043C2D0`, `FUN_0065ADF0 @ 0x0065ADF0` | exact `DynamicVectorClass__Contains` body not separately decompiled here |
| Full contact/no-slot standard branch | verified | `0x0043C2D0`; branch continues to `0x13`/`0x12` | caller-side global queue timing deferred |
| `NEED_TO_MOVE(0x13)` early return | verified | `0x0043C2D0`; `FootClass::Receive_Radio @ 0x004D8FB0` | local sentinel identity deferred |
| `MOVE_TO_CELL(0x12)` not-arrived return | verified | `0x0043C2D0`; `0x004D8FB0` case `0x12` | exact pathfinding movement timing deferred |
| `QueueingCell=4,1` use in receiver `0x0E` | verified negative | `0x0043C2D0`; `artmd.ini` | later unit-side use deferred |
| `FootClass::Mission_Enter @ 0x004D9290` caller handling | touched-not-exhausted | decompile `0x004D9290` | full state machine outside slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is this code active for stock YR refinery docking? -> Yes; GAREFN/NAREFN have DockUnload=yes/NumberOfDocks=1 and CMIN/HARV dock to them.` (evidence: `rulesmd.ini`, `0x0043C2D0`)
- `[RESOLVED] OQ-2 - Does occupied/no-free contact cause immediate `0x0E` NEGATORY? -> No in the standard DockUnload receiver branch; the function continues to `0x13` and `0x12` unless a prior hard gate fired.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-3 - What does `FUN_0065ADF0` mean? -> It scans `Contacts[]` and returns low byte true only for an empty slot or matching sender.` (evidence: `0x0065ADF0`)
- `[RESOLVED] OQ-4 - Does `QueueingCell` feed this non-accepted receiver path? -> No; `0x0E` uses hardcoded `(NW+3,NW+1)` for the `0x12` payload and does not read `+0x1618/+0x161C`.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-5 - What happens when the requester is still moving/chrono-staged? -> `0x13` can return `10`; building returns `1` before assigning `0x12`, so no `0x18/0x16`.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-6 - What happens when requester is not already at the hardcoded cell? -> `0x12` sets destination/timestamp and returns `1`; building returns `1` without `0x18/0x16`.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-7 - What does Mission_Enter do with a non-hard-reject result? -> `0x004D9290` treats return `1` as success/continue; hard non-`1` can trigger BREAK and destination clear when `+0x418` is false.` (evidence: `0x004D9290`)
- `[DEFERRED] OQ-8 - Exact global queue/side-step timing after the previous miner exits.` (category: requires-different-system-context; reason: belongs to unit-side Mission_Enter/unload/exit handoff, not receiver `0x0E`; next-step-if-pursued: trace the two-miner queue from `Mission_Enter` through stock zero-link exit.)
- `[DEFERRED] OQ-9 - Runtime identity of the decompiler local sentinel used in the `0x13` branch.` (category: bounded-cost-too-high; reason: not needed to distinguish receiver outcomes; next-step-if-pursued: disassemble the local stack setup around the `0x13` call.)
- `[DEFERRED] OQ-10 - Whether `QueueingCell=4,1` is consumed by later unit mission or only by Rust staging abstractions.` (category: out-of-scope; reason: the scoped receiver function has a verified negative; next-step-if-pursued: investigate unit-side far/waiting path call chain.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Busy/not-yet-arrived receiver outcome returns `1` after `0x12` returns `1`; no `0x18/0x16` is sent until `0x12` returns `0x14`. | `0x0043C2D0`; `0x004D8FB0` case `0x12` | partial/mismatch risk: Rust `try_reserve=false` models an occupied dock as queued reservation failure, not a radio `ROGER` with no dock-enter burst | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs` | Distinguish "assigned wait/approach target" from "dock-enter accepted"; do not promote to dock/unload until at the `CAN_DOCK` target. | Two miners, one refinery: second miner receives/keeps a wait/approach target but does not enter unload while first is active. Proposed test: `second_miner_can_dock_roger_without_enter_dock_until_already_at_target` | Do not encode a receiver-side hard `NEGATORY` for normal occupied DockUnload. |
| `NEED_TO_MOVE(0x13)` can stop the burst early with building return `1` when the unit is still in chrono/moving state. | `0x0043C2D0`; `0x004D8FB0` case `0x13`; CMIN `Teleporter=yes` | unchecked | `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, teleport movement state | A chrono miner still resolving a move/teleport should not receive dock-enter/unload just because it has contacted a refinery. | Chrono miner with active inbound movement asks `CAN_DOCK`; dock phase remains approach/wait, not linked/unloading. Proposed test: `chrono_miner_need_to_move_reply_defers_enter_dock_burst` | Do not skip directly to `MissionEnter`/unload on contact while teleport/move state is active. |
| Receiver `0x0E` never reads art `QueueingCell`; it uses hardcoded `(NW+3,NW+1)` for its `0x12` target even when not finally accepted. | `0x0043C2D0`; `artmd.ini` `QueueingCell=4,1` | mostly matched for `refinery_can_dock_queue_cell`; separate Rust `wait_queue` still uses art `QueueingCell` | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs` | Keep `QueueingCell` out of the receiver-equivalent `CAN_DOCK` target; if using art `QueueingCell` for far/wait staging, document/test it as a separate unit-side behavior. | GAREFN at `(rx,ry)`: non-arrived `CAN_DOCK` assignment target is `(rx+3,ry+1)`; art wait/staging, if used, is not called the accepted receiver target. Proposed test: `can_dock_not_arrived_uses_hardcoded_refinery_target_not_art_queueing_cell` | Do not merge `QueueingCell=4,1` into the `0x0E` accepted or not-yet-arrived `0x12` payload. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` section 6.1 uses wording that says "if InDockQueue(sender)" before the hardcoded cell calculation. Replacement wording: "The function checks contact membership and may try HELLO if a free contact exists, but the standard DockUnload `0x13`/`0x12` sequence is not guarded by a final `InDockQueue(sender)` hard reject; no-slot/no-contact can still return `1` after direct `0x12` assignment/deferral."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/MINER_DOCK_QUEUE_TWO_MINERS_ONE_REFINERY_TRACE.md` stage 5 says `0x0E` returns NEGATORY when no slot is available. Replacement wording: "`HELLO(0x02)` can reject when Contacts[] is full, but `BuildingClass::Receive_Radio` case `0x0E` standard DockUnload does not itself hard-return `10` merely because no free contact slot exists; it can still send `0x13`/`0x12` and return `1` without `0x18/0x16`."

## 10. Negative Facts / Do Not Do

- Do not model standard occupied DockUnload `0x0E` as immediate `NEGATORY(10)` solely due to no free contact slot. Evidence: `0x0043C2D0` continues to `0x13`/`0x12`; hard `10` is from power or special gates.
- Do not send `ENTER_DOCK(0x18)` or `TIMING_SYNC(0x16)` after a `MOVE_TO_CELL(0x12)` reply of `1`. Evidence: `0x0043C2D0` returns `1` unless reply is `0x14`.
- Do not use `QueueingCell=4,1` for this receiver's `0x12` target. Evidence: `0x0043C2D0` hardcodes `(NW+3,NW+1)`; no `+0x1618/+0x161C` read.
- Do not treat `BuildingClass+0x2E4` as written by this `0x0E` path. Evidence: live decompile has no write; corroborated by prior zero-link DockUnload reports.
- Do not use `GetDockCellForObject @ 0x0044EFB0` for this receiver target. Evidence: no call from `0x0043C2D0`; prior reports identify it as production exit oracle.

## 11. Remaining Uncertainty

- Exact later queue/side-step timing for a second miner waiting while the first exits remains outside this receiver slice.
- Exact runtime identity of the `0x13` branch's local sentinel is deferred; the observable branch outcome is still verified.
- Whether stock binary consumes `QueueingCell=4,1` in a later unit-side far/waiting path remains outside this report; this report only proves `0x0043C2D0` does not consume it.

## Sources

- Ghidra live decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra live decompile: `FUN_0065ADF0 @ 0x0065ADF0`
- Ghidra live decompile: `RadioClass::Receive_Radio @ 0x0065A820`
- Ghidra live decompile: `RadioClass::Transmit_Radio @ 0x0065AAA0`
- Ghidra live decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`
- Ghidra live decompile: `FootClass::Receive_Radio @ 0x004D8FB0`
- Ghidra live decompile: `FootClass::Mission_Enter @ 0x004D9290`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
