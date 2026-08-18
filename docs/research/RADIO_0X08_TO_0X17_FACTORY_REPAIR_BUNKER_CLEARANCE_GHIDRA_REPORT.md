# RADIO_0X08_TO_0X17 Factory/Repair/Bunker Clearance - Ghidra Research Report

Date: 2026-05-22  
Target: `RADIO_0X08_TO_0X17_FACTORY_REPAIR_BUNKER_CLEARANCE`  
Status: PARTIAL  
Scope: active factory, repair, and bunker uses of BuildingClass/TechnoClass radio `0x08` and any `0x17` queued/clearance response. Stock refinery DockUnload queue admission is explicitly excluded.

## Session Limitation

No live Ghidra MCP tools were exposed in this subagent slot. This report is therefore a synthesis over existing Ghidra-backed reports and local source/INI scans, not a fresh decompile session. Handoff-critical claims below cite the prior report evidence that included decompile, assembly, caller, or INI/default proof. Any claim requiring a new caller sweep is marked unresolved.

## Target Question

Which active YR systems outside stock refinery DockUnload use the `BuildingClass::Receive_Radio(0x08)` path that can reply `0x17`, what does that reply mean, and what should Rust preserve or avoid when implementing factory, repair, and bunker clearance?

## Non-Goals

- Do not re-litigate stock `GAREFN`/`NAREFN` refinery DockUnload behavior.
- Do not prove the full `0x0E`/`0x0F` dock reservation protocol.
- Do not reverse the complete war factory door/exit state machine.
- Do not reverse the complete repair depot or bunker occupant state machines.
- Do not edit Rust, INI, or in-repo documentation.

## Evidence Needed To Mark COMPLETE

- Decompile plus disassembly range for `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x08`, including the `WeaponsFactory`, `UnitRepair`, and `Bunker` gates and the near-distance shortcut.
- Decompile plus disassembly range for `TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x08`, including its directed `0x19` then `0x03` cleanup.
- Sender/caller evidence for a live unit path that transmits `0x08` to one of those building classes and compares the returned value to `0x17`.
- INI/default evidence that stock YR actually has active `WeaponsFactory=yes`, `UnitRepair=yes`, and `Bunker=yes` building types.
- Receiver evidence for `0x17` when it is sent as a message, so the reply code is not confused with a transmitted message.

## Stop Conditions

- Stop once receiver semantics, known sender evidence, active stock flags, Rust implications, and negative facts are documented.
- Stop rather than expanding into stock refinery, aircraft, full war factory exit, or complete repair/bunker lifecycle research.
- Stop and mark PARTIAL if fresh Ghidra decompile/caller sweep is unavailable.

## Coverage Ledger

| Question | Status | Evidence |
|---|---:|---|
| Does `BuildingClass::Receive_Radio(0x08)` have a `0x17` reply branch? | Resolved | Prior Ghidra-backed `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE_HANDOFF_EXIT_GHIDRA_REPORT.md`. |
| Which flags gate the `0x17` reply? | Resolved | Same reports: `WeaponsFactory`, `UnitRepair`, `Bunker`. |
| Is the branch active in stock YR data? | Resolved for receiver flags | Stock `rulesmd.ini` sections include land war factories, service depots, and `NATBNK`. |
| Is `0x17` a reply or a sent message here? | Resolved | Prior radio protocol and Foot/Unit receiver reports separate return value from sent message handling. |
| Which exact factory/repair/bunker runtime sender uses it? | PARTIAL | Generic `UnitClass::Mission_Enter @ 0x00739EC0` sender evidence exists; fresh per-system caller/runtimes were not available in this slot. |

## Verified Binary Findings

### 1. BuildingClass radio `0x08` replies `0x17` only for factory/repair/bunker receivers

Active in YR: Conditional.

Existing Ghidra-backed reports identify `BuildingClass::Receive_Radio @ 0x0043C2D0`, vtable `+0x194`, with explicit switch cases including `0x08`. The case `0x08` behavior is:

```text
if Type.UnitRepair or Type.Bunker:
    if distance(this.center, sender.center) < 0x180:
        return 1
TechnoClass::Receive_Radio(sender, 0x08, payload)
if not (Type.WeaponsFactory or Type.UnitRepair or Type.Bunker):
    return 1
return 0x17
```

Evidence: `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`; `BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE_HANDOFF_EXIT_GHIDRA_REPORT.md`; `miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`.

Interpretation: `0x17` is a queued/deferred-clearance reply for buildings with `WeaponsFactory=yes`, `UnitRepair=yes`, or `Bunker=yes`. It is not the stock refinery queue-admission result; stock refinery receivers fall through to `return 1` after Techno cleanup because they do not satisfy these three gates.

### 2. UnitRepair/Bunker have a near-distance shortcut that bypasses `0x17`

Active in YR: Yes for building types with `UnitRepair=yes` or `Bunker=yes`; branch is conditional on distance.

The `UnitRepair`/`Bunker` pre-check happens before the call to `TechnoClass::Receive_Radio(0x08)`. If sender and building centers are closer than `0x180` leptons, the building immediately returns `1`. The corrected scale is `0x180 = 384` leptons, or 1.5 cells at 256 leptons per cell. Older wording that called this "3 cells" is stale.

Evidence: same BuildingClass reports, with corrected scale in `miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`.

Interpretation: for repair depots and bunkers, `0x08 -> 0x17` is not guaranteed. A close sender gets immediate acceptance (`1`), while a farther sender can pass through Techno cleanup and receive queued/deferred (`0x17`) if the type flag matches.

### 3. TechnoClass radio `0x08` is cleanup, not queue admission

Active in YR: Conditional on callers reaching TechnoClass through class fallthrough or explicit base call.

Prior Ghidra-backed reports identify `TechnoClass::Receive_Radio @ 0x006F4AB0`. Case `0x08` sends directed `0x19` to the sender, then sends `0x03` (`BREAK`) to the sender, and returns the `0x03` result. In the BuildingClass `0x08` path, this cleanup runs before the final factory/repair/bunker `0x17` test unless the UnitRepair/Bunker near-distance shortcut returns early.

Evidence: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`; `miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`; `BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE_HANDOFF_EXIT_GHIDRA_REPORT.md`.

Interpretation: Rust should not treat `0x08` as a pure query. On the real receiver path it can actively clear or break the sender's old radio relationship before reporting queued/deferred for factory/repair/bunker types.

### 4. A generic UnitClass sender transmits `0x08` and checks for `0x17`

Active in YR: Yes for the generic mission path; exact factory/repair/bunker scenario coverage remains PARTIAL.

Existing Ghidra-backed Mission_Enter audit identifies a radio send in `UnitClass::Mission_Enter @ 0x00739EC0`:

```text
0x0073A939  PUSH 0x8
            CALL [EDX+0x274]
0x0073A943  compare return to 0x17
```

Evidence: `miner/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`.

Interpretation: the sender side has an explicit "send request-clearance `0x08`, then branch on queued reply `0x17`" path. The receiver evidence shows that the `0x17` branch is produced by `WeaponsFactory`, `UnitRepair`, and `Bunker` receivers, not by stock refinery receiver flags. This report does not freshly prove which player-visible factory/repair/bunker action enters that sender path in every case.

### 5. `0x17` reply is distinct from sent radio message `0x17`

Active in YR: Yes as a sent message handler, but separate from the BuildingClass `0x08` reply.

`FootClass::Receive_Radio @ 0x004D8FB0` has an explicit `0x17` case at `0x004D902B`. It clears path/destination state under guarded conditions, changes mission `0` or `7` toward Guard (`5`), may trigger fire/do-action behavior, may perform a locomotor reset with a null CLSID when not chrono, and then falls through to TechnoClass.

`UnitClass::Receive_Radio @ 0x00737430` also has case `0x17`; its special body is conditional on Weeder/adjacent flags and deploy flag, then it falls through to FootClass behavior.

Evidence: `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`; `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`; `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`.

Interpretation: a returned value `0x17` from BuildingClass means queued/deferred clearance. A transmitted message `0x17` is an eviction/path-clear style message handled by Foot/Unit/Aircraft. These must not be collapsed into one Rust event.

## Active Stock YR Receiver Data

Active in YR: Yes for receiver flags.

Existing INI/default evidence shows stock YR building types with the receiver gates:

- `GAWEAP`, `NAWEAP`, `YAWEAP`: `WeaponsFactory=yes`, `Factory=UnitType`.
- `GADEPT`, `NADEPT`, `YADEPT`, `CAOUTP`: `UnitRepair=yes`, `NumberOfDocks=1`.
- `NATBNK`: `Bunker=yes`, `NumberOfDocks=1`.

Evidence: prior war factory, UnitRepair, and bunker Ghidra reports with `rulesmd.ini` line references; local scan confirms these sections and keys are present.

## Factory, Repair, Bunker Interpretation

### War Factories

Active in YR: Conditional.

The receiver gate is active because stock land war factories set `WeaponsFactory=yes`. However, existing war factory reports show initial stock land vehicle production exit uses a separate spawn/unlimbo path: it places the produced unit at `ExitCoord`, sends `HELLO(2)` and `0x18`, creates reciprocal radio contact, and relies on the contact-sensitive `NumberImpassableRows` branch while the vehicle drives out. Those reports did not prove that final drive-out clearance sends `0x08`.

Conclusion: implement the receiver semantics when modeling radio protocol, but do not reinterpret the initial war factory spawn as a `0x08 -> 0x17` admission queue.

### Repair Depots

Active in YR: Conditional.

Repair depots have the receiver gate through `UnitRepair=yes`. The `0x08` case can return `1` immediately when the sender is within 1.5 cells, or return `0x17` after Techno cleanup when farther away. Existing repair mission reports emphasize `0x13` and `0x1C` during repair processing and do not prove a BuildingClass state-machine sender that transmits `0x17`.

Conclusion: repair depot approach/clearance should preserve the `0x08` receiver shortcut and queued reply semantics, while repair tick behavior remains governed by the separate repair mission/radio messages.

### Bunkers

Active in YR: Conditional.

Bunkers have the receiver gate through `Bunker=yes`, and `NATBNK` is active in stock YR. The same near-distance shortcut applies. Existing bunker reports show occupant linkage in the bunker state machine, including reciprocal occupant/building fields and `BunkerWallsUpSound`, but do not show that the bunker state machine sends `0x17` as a message.

Conclusion: bunker entry/approach logic should not skip the `0x08` receiver semantics, but `0x17` must remain a queued reply unless a separate sent-message caller is proven.

## Rust Surface Scan

Rust currently does not have a general RadioClass protocol model matching message IDs `0x08` and `0x17`. Relevant existing abstractions are direct state fields and reservation systems:

- `src/sim/game_entity.rs`: `radio_contacts`, `bunker_occupant`, and contact helper methods.
- `src/sim/production/production_spawn.rs`: war factory spawn contact helpers and exact land vehicle exit placement.
- `src/sim/production/production_queue.rs`: calls the war factory contact marking path.
- `src/sim/docking/building_dock.rs`: repair depot dock/reservation state.
- `src/sim/production/production_types.rs`: dock reservation structures.
- `src/sim/pathfinding/cell_entry.rs` and `src/sim/pathfinding/movement_occupancy.rs`: UnitRepair/Bunker contact-sensitive cell-entry behavior.

## Implementation Handoff

- Model `BuildingClass::Receive_Radio(0x08)` receiver semantics by building type flags: `WeaponsFactory`, `UnitRepair`, and `Bunker` can reply `0x17`; nonmatching buildings reply `1` after Techno cleanup. Keep stock refineries out of this queued reply path.
- For `UnitRepair`/`Bunker`, preserve the near-distance early `return 1` for `< 0x180` leptons before Techno cleanup and queued reply evaluation.
- Keep response code `0x17` separate from sent message `0x17`. The reply means queued/deferred clearance; the sent message means path/mission eviction behavior handled by Foot/Unit/Aircraft receivers.

Concrete Rust test-name proposal:

```text
building_radio_0x08_factory_repair_bunker_returns_queued_not_refinery
```

## Negative Facts / Do Not Do

- Do not route stock `GAREFN`/`NAREFN` DockUnload queue admission through `0x08 -> 0x17`.
- Do not treat `0x08` as read-only; `TechnoClass::Receive_Radio(0x08)` sends `0x19` then `0x03` cleanup.
- Do not convert returned `0x17` into a transmitted `0x17` event unless a sender explicitly sends message `0x17`.
- Do not apply the `0x17` BuildingClass reply to all buildings; it is gated by `WeaponsFactory`, `UnitRepair`, or `Bunker`.
- Do not use the stale "0x180 = 3 cells" distance. The corrected value is 384 leptons, 1.5 cells.

## Remaining Uncertainty

- Fresh live Ghidra decompile/caller sweep was unavailable in this slot.
- Exact player-visible frequency and state path for `UnitClass::Mission_Enter @ 0x00739EC0` against each stock war factory, repair depot, and bunker receiver was not freshly traced.
- Existing war factory reports still defer the final drive-out/contact-clear sender; this report does not prove a war-factory-specific final `0x08` transmit.
- Sent-message `0x17` callers outside the documented Foot/Unit/Aircraft receiver behavior were not exhaustively swept.

## Stale-Doc Wording

Suggested replacement for broad wording such as "Case 0x08 fires during any factory-exit or repair-dock approach":

```text
BuildingClass radio case 0x08 is active for cleanup/clearance contexts. After optional UnitRepair/Bunker near-distance acceptance and TechnoClass 0x19 + 0x03 cleanup, it returns queued/deferred code 0x17 only for WeaponsFactory, UnitRepair, or Bunker receivers. Stock GAREFN/NAREFN refineries do not use this 0x17 reply for DockUnload queue admission.
```

Suggested replacement for stale distance wording:

```text
0x180 is 384 leptons, equal to 1.5 cells at 256 leptons per cell, not 3 cells.
```

## Status

PARTIAL. Receiver semantics, active stock receiver flags, response/message distinction, Rust implications, and key negative facts are documented from prior Ghidra-backed evidence. Fresh live Ghidra decompilation and a per-system sender/caller sweep were not available in this subagent slot.
