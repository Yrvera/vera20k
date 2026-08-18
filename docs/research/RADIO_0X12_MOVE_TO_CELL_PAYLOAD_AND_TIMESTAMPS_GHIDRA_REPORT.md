# RADIO 0x12 MOVE_TO_CELL Payload And Timestamps - Ghidra Research Report

**Date:** 2026-05-22  
**Target:** `RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS`  
**Primary addresses:** `FootClass::Receive_Radio @ 0x004D8FB0`, `BuildingClass::Receive_Radio @ 0x0043C2D0`, `UnitClass::Receive_Radio @ 0x00737430`, `RadioClass::Transmit_Radio_Impl @ 0x0065A970`  
**Investigation mode:** Focused static binary/decompile slice. Local Ghidra MCP was not exposed in this session, so this report uses the local decompiler export plus direct disassembly of the retail `gamemd.exe`.  
**Active in YR:** Yes for standard refinery admission. Conditional for helipad and unit-side docking/loading contexts.  
**Confidence:** High for the standard refinery sender, FootClass receiver, payload shape, return codes, and `+0xC8..+0xD0` writes. Medium for null-payload and non-refinery sender completeness.

## 0. Scope Contract

**Target question:** Decode radio message `0x12` move-to-cell behavior across known senders, payload shape, receiver behavior, already-at-cell return, and `FootClass+0xC8..+0xD0` timestamp writes. Determine Rust movement/dock handoff implications.

**Non-goals:**

- Do not redo the full `FootClass::Receive_Radio` switch.
- Do not reclassify every radio opcode.
- Do not implement Rust changes.
- Do not broaden into full carryall, transport, airfield, or factory radio behavior except where it proves a `0x12` sender or receiver path.

**Evidence needed to mark COMPLETE:**

- Decompile plus disassembly for `FootClass::Receive_Radio` case `0x12`.
- Caller/sender evidence for every scoped `0x12` transmit site found in `BuildingClass::Receive_Radio` and `UnitClass::Receive_Radio`.
- Disassembly showing payload storage immediately before the `0x12` transmit.
- Disassembly showing already-at-cell return and the `+0xC8/+0xCC/+0xD0` writes.
- Rust handoff mapped to current miner/passenger movement surfaces without changing code.

**Stop conditions:**

- Stop after proving the standard refinery path, the local sender set found by bounded `0x12` transmit scan, and the receiver write semantics.
- Stop if a path requires runtime watchpoints or broader global radio archaeology.
- Stop before editing Rust, INI files, or existing in-repo docs.

## 1. Executive Summary

Radio `0x12` is the FootClass `MOVE_TO_CELL` assignment used by standard refinery admission. It is sent synchronously through `RadioClass::Transmit_Radio_Impl`; there is no deferred radio queue in this slice.

For standard refinery/weeder admission, `BuildingClass::Receive_Radio` case `0x0E` sends `0x12` to the contacted unit with a `CellClass*` payload for the building northwest cell plus `(3,1)`. The FootClass receiver compares that payload target cell with the receiver's current cell. If already there, it returns `0x14` and does not set a new destination or write the mission timer triplet. If not already there, it sets the destination, writes `+0xC8 = current frame`, `+0xCC = target coord Y` from the local coordinate buffer in the standard non-null path, `+0xD0 = 0`, and returns `1`.

The building-side consequence is important: standard refinery admission only proceeds to the later `0x18` and `0x16` messages when the `0x12` reply is `0x14`. A reply of `1` means "move assigned"; it is not accepted as "already at dock cell" for the follow-up enter/unload messages.

## 2. Radio Dispatch Primitive

`RadioClass::Transmit_Radio_Impl @ 0x0065A970` dispatches non-special radio messages by filtering the sender/contact and calling the target receiver vtable slot `+0x194`.

Disassembly evidence:

- `0x0065A970..0x0065AA70`: general radio transmit implementation.
- `0x0065A9DB`: calls `[target_vtable+0x194]` for normal messages.
- HELLO/BREAK have contact management branches, but scoped `0x12` uses the normal synchronous receiver call.

**Verified finding:** `0x12` is a synchronous call/return handoff in this slice. Rust should not model it as a delayed independent radio packet unless a higher-level system explicitly schedules the retry.

## 3. FootClass Receiver Case 0x12

Decompile at `FUN_004d8fb0` case `0x12`:

```c
case 0x12:
  if ((int *)*param_4 != (int *)0x0) {
    piVar6 = (int *)(**(code **)(*(int *)*param_4 + 0x48))(local_c);
    iVar5 = *piVar6;
    iVar1 = piVar6[1];
    psVar7 = (short *)(**(code **)(*param_1 + 0x1b8))(&param_2);
    if ((*psVar7 == (short)(iVar5 + (iVar5 >> 0x1f & 0xffU) >> 8)) &&
        (psVar7[1] == (short)(iVar1 + (iVar1 >> 0x1f & 0xffU) >> 8))) {
      return 0x14;
    }
  }
  iVar5 = (**(code **)(*param_1 + 0x184))();
  if ((iVar5 == 5) && (param_1[0x2d] == -1)) {
    (**(code **)(*param_1 + 0x1e8))(2,0);
  }
  if ((param_1[0x2d] == 7) && (cVar3 = (**(code **)(*param_1 + 0x200))(), cVar3 != '\0')) {
    (**(code **)(*param_1 + 0x1ec))();
  }
  (**(code **)(*param_1 + 0x480))(*piVar2,1);
  param_1[0x32] = DAT_00a8ed84;
  param_1[0x33] = iStack_10;
  param_1[0x34] = 0;
  return 1;
```

Because `param_1` is an `int*`, the key fields are:

| Field | Meaning in this slice | Evidence |
|---|---|---|
| `this+0xB4` | `MissionClass::QueuedMission` | `param_1[0x2d]`; resolved by `0x005B2DA0`, `0x005B3040`, `0x005B35E0` in prior focused reports |
| `this+0xC8` | mission dispatch timer start frame | `param_1[0x32] = DAT_00a8ed84` |
| `this+0xCC` | middle dword of dispatch timer storage; source is target coord Y in standard non-null path | `param_1[0x33] = iStack_10`; disassembly source `[ESP+0x20]` |
| `this+0xD0` | mission dispatch timer duration | `param_1[0x34] = 0` |

Disassembly range `0x004D9139..0x004D9210`:

- `0x004D914B`: call payload vtable `+0x48` to get target coordinates into the local stack buffer.
- `0x004D9150..0x004D9177`: convert X/Y leptons to cell coordinates with `(coord + ((coord >> 31) & 0xFF)) >> 8`.
- `0x004D917A`: call receiver vtable `+0x1B8` to get current cell.
- `0x004D9180..0x004D918E`: if current cell equals target cell, return `0x14`.
- `0x004D919A..0x004D91BA`: if current mission is `5` and queued mission is `-1`, queue mission `2`.
- `0x004D91C0..0x004D91DB`: if queued mission is `7` and the resume predicate succeeds, resume mission.
- `0x004D91EB`: call receiver vtable `+0x480` with `(*payload, 1)` to set destination.
- `0x004D91F1`: load `g_CurrentFrameCounter @ 0x00A8ED84`.
- `0x004D91F6`: load `[ESP+0x20]`, the Y dword from the target coordinate buffer in the standard non-null path.
- `0x004D9203`: write `this+0xC8`.
- `0x004D920A`: write `this+0xCC`.
- `0x004D920D`: write `this+0xD0 = 0`.

**Verified finding:** The already-at-cell return `0x14` exits before mission queuing, destination assignment, and `+0xC8..+0xD0` writes.

**Verified finding:** In the standard non-null payload path, `+0xCC` is not an independent chrono-miner timestamp. It receives the target coordinate Y dword from the local coordinate buffer, while `+0xD0` is the dispatch wait duration and is cleared to zero.

## 4. Sender Set And Payload Shapes

### 4.1 BuildingClass case 0x0E: standard refinery/weeder path

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` is the stock refinery admission sender for `0x12`.

Disassembly range `0x0043C9F5..0x0043CADB`:

- `0x0043C9F5`: send `0x13` to the sender first.
- `0x0043CA13`: store `this` into the payload local for one branch.
- `0x0043CA1B..0x0043CA2F`: read building type flags including `DockUnload`, `Weeder`, and `Helipad`.
- `0x0043CA71..0x0043CA92`: for refinery/weeder, get building cell and compute a packed cell with X plus `3` and Y plus `1`.
- `0x0043CAA4`: call map cell lookup helper `0x005657A0`.
- `0x0043CAAE`: store returned `CellClass*` into the payload local.
- `0x0043CAB4..0x0043CAB8`: send radio `0x12` with that payload.
- `0x0043CABE..0x0043CAC1`: compare reply to `0x14`; if not `0x14`, leave without sending later dock messages.
- `0x0043CACA..0x0043CAD7`: only after `0x14`, send `0x18` and `0x16`.

**Payload shape:** `CellClass*` for `building_nw_cell + (3,1)` in the standard refinery/weeder branch.

**Active in YR:** Yes for standard refineries. INI evidence: `[GAREFN] DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` at `ini/rulesmd.ini:11722..11729`; `[NAREFN] DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` at `ini/rulesmd.ini:12515..12521`; `[CMIN] Dock=NAREFN,GAREFN` at `ini/rulesmd.ini:7361`; `[NAREFN]` and `[GAREFN]` `QueueingCell=4,1` at `ini/artmd.ini:1716` and `1773`.

### 4.2 BuildingClass case 0x0E: helipad branch

The same building case has a helipad branch.

Disassembly range `0x0043CA3D..0x0043CA55`:

- Store payload as `this` building pointer.
- Send `0x12` to the radio sender.
- Require reply `0x14` before the branch continues to the later radio step.

**Payload shape:** `BuildingClass*` (`this`) for the helipad branch.

**Active in YR:** Conditional. The code is live when the building type is a helipad-style dock provider, but it is not the standard refinery `DockUnload=yes` miner path.

### 4.3 UnitClass case 0x0E sender

`UnitClass::Receive_Radio @ 0x00737430` case `0x0E` also sends `0x12` after a `0x13` gate.

Disassembly range `0x00737A55..0x00737A83`:

- `0x00737A55`: send `0x13` to the sender.
- `0x00737A67`: store `this UnitClass*` in the payload local.
- `0x00737A6D..0x00737A71`: send radio `0x12`.
- `0x00737A77`: compare reply to `1`.
- `0x00737A7F`: if reply is not `1`, send `0x03` break/abort.

**Payload shape:** `UnitClass*` (`this`) for this unit-side docking/loading context.

**Active in YR:** Conditional. The binary path is live, but it is not the standard refinery building admission branch. It matters for generic `0x12` modeling because the payload is object-like, not always `CellClass*`.

### 4.4 Receiver class fan-out

Existing verified switch reports establish:

- `UnitClass::Receive_Radio` does not directly handle `0x12`; it falls through to `FootClass::Receive_Radio`.
- `AircraftClass::Receive_Radio` can gate specific missions, but scoped non-gated `0x12` behavior falls through to FootClass machinery.
- Infantry uses the FootClass receiver path.
- `BuildingClass::Receive_Radio` has no positive direct `0x12` move receiver in its direct case set; building receiver behavior is not the FootClass move-to-cell behavior.

## 5. Already-At-Cell Return Contract

The `0x14` return is not a generic "accepted movement order" reply. In this slice it specifically means the receiver is already on the target cell computed from the payload target's coordinates.

Standard refinery consequence:

1. Building receives the unit's `0x0E` docking/admission request.
2. Building sends `0x13`.
3. Building sends `0x12` with `CellClass*` payload for NW plus `(3,1)`.
4. If FootClass returns `1`, the unit has been assigned movement; building does not send `0x18` or `0x16` in that pass.
5. If FootClass returns `0x14`, the unit is already at the accepted cell; building then sends `0x18` and `0x16`.

**Verified finding:** Rust docking logic should treat "assigned move to accepted cell" and "already at accepted cell, proceed with dock handoff" as separate states.

## 6. Rust Implementation Handoff

Current Rust has miner-specific movement/dock state machinery rather than a generic radio payload dispatcher:

- `src/sim/miner/miner_dock_sequence.rs` computes a wait/queue cell from art `QueueingCell` but computes the accepted can-dock cell as `refinery_nw + (3,1)`.
- `phase_mission_enter` issues a direct move to the accepted cell when not already there, then waits in `AwaitingAcceptedCell`.
- `phase_awaiting_accepted_cell` returns to `MissionEnter` after movement completes so the next admission pass can perform the already-at-cell handoff.
- Passenger/transport logic in `src/sim/passenger.rs` has direct boarding phases and no generic `0x12` radio payload dispatcher.

Handoff requirements:

- Preserve the refinery distinction between `0x12` reply `1` and `0x14`: reply `1` assigns movement and defers `0x18/0x16`; reply `0x14` allows enter/unload follow-up.
- Do not derive the accepted docking cell from art `QueueingCell`. The standard refinery `0x12` payload is the hardcoded building NW plus `(3,1)` cell; `QueueingCell=4,1` is a separate waiting/staging target.
- If a generic radio layer is added later, make `0x12` payload object-like: standard refinery sends `CellClass*`, helipad sends `BuildingClass*`, and UnitClass case `0x0E` sends `UnitClass*`. The receiver asks the payload for coordinates.

Concrete Rust test proposal:

`radio_0x12_move_to_cell_reply_one_defers_enter_until_already_at_accepted_cell`

This should assert that a refinery admission pass which assigns movement to NW plus `(3,1)` does not start the `0x18/0x16`-equivalent dock handoff until a later pass sees the unit already at that accepted cell.

## 7. Negative Facts / Do Not Do

- Do not treat radio `0x11` as move-to-cell. The settled move assignment is `0x12`.
- Do not treat `0x12` return `1` as "already accepted for enter dock"; for standard refinery, only `0x14` opens the later `0x18/0x16` sends.
- Do not use art `QueueingCell=4,1` as the `0x12` accepted cell. It is not read by the standard building case `0x0E` receiver slice that sends `0x12`.
- Do not model `+0xCC` as a standalone chrono miner timing source in this path. The checked dispatch wait uses `+0xC8` and `+0xD0`; `+0xD0` is cleared to zero by `0x12`.
- Do not assume the `0x12` payload is always a cell pointer. Standard refinery uses `CellClass*`, but helipad and UnitClass paths send object pointers.

## 8. Stale Or Superseded Wording

`FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT` is stale on `this+0xB4` naming. Its switch addresses and case set remain useful, but phrases such as "team-related field or secondary mission state", "likely TeamID or SubMission", and "no team" should be replaced with:

> `this+0xB4` is `MissionClass::QueuedMission`; `-1` means no queued mission, and value `7` is queued Mission 7 in the checked radio cases.

The same older report is also superseded on `iStack_10`/`+0xCC` source. Its "likely high word of a 64-bit counter" wording should be replaced with:

> In FootClass case `0x12` standard non-null payload path, `+0xCC` receives the target coordinate Y dword from the local coordinate buffer at `[ESP+0x20]`.

## 9. Remaining Uncertainty

- No live Ghidra MCP or runtime watchpoint was available in this slot; the findings are based on static decompile export plus direct disassembly of the retail executable.
- The value written to `+0xCC` for a null `*payload` caller remains unresolved. The standard refinery sender provides a non-null payload.
- The bounded sender set covers explicit `0x12` sends in the scoped building and unit docking paths; a full global arbitrary-message broadcast audit was intentionally out of scope.
- The exact player-visible use of the UnitClass `0x0E -> 0x12` sender outside refinery admission remains conditional.

## 10. Status

**COMPLETE** for the scoped standard refinery `0x12` sender/receiver contract, already-at-cell return, payload shape, timestamp writes, and Rust handoff.
