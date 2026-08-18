# Radio Message 0x11 Senders And Meaning - Ghidra Research Report

Date: 2026-05-22  
Target: `RADIO_MSG_0X11_SENDERS_AND_MEANING`  
Binary: `gamemd.exe`  
Status: COMPLETE  
Active in YR: Conditional yes

## 0. Investigation Contract

### Target Question

Find every live sender/caller of radio message `0x11` in `gamemd.exe` and determine the canonical behavior/name from call context. Focus on whether it is a drive/nav/dock instruction, which classes receive it, and whether it is active in standard Yuri's Revenge.

### Non-goals

- Do not redo the whole `RadioClass` primitive map.
- Do not investigate unrelated radio messages except to distinguish `0x11` from adjacent movement messages.
- Do not implement Rust changes.
- Do not patch in-repo docs or INI files.

### Evidence Needed To Mark COMPLETE

- Exhaustive radio-transmit call-site scan for literal `0x11` senders across the known transmit vtable slots.
- Decompiled sender and assembly range for the live sender.
- Decompiled receiver handling and assembly range for the positive `0x11` response.
- Class fan-out evidence for which classes inherit or override the handling.
- INI/default source evidence for any sender gates that determine whether the path can be active in standard YR.

### Stop Conditions

- Stop once all known `RadioClass` transmit helper slots have been scanned for `0x11`.
- Stop once the one live sender's gate, payload, receiver expectation, and active-YR status are established.
- Do not chase visual consequences of the deploy/door timer beyond identifying its role as the sender gate.

## 1. Executive Summary

Verified binary finding: radio message `0x11` is not a drive/nav/dock instruction. It is a transport passenger entry status poll sent by `UnitClass::AI` through `Transmit_Radio_ToFirst` when a transport-like unit with `Passengers > 0` has an active deploy/door animation tracker and is not already in mission `0x10`.

The only verified positive receiver behavior is in `FootClass::Receive_Radio`: for `0x11`, it returns `1` only when the receiver's current mission or queued mission is `7`, the already-documented `Mission_Enter` path. Otherwise it falls through to `TechnoClass::Receive_Radio`, where `0x11` is not handled.

Canonical behavior-derived name: `ARE_YOU_ENTERING` or `ENTERING_STATUS_POLL`. The message asks the first radio contact whether it is currently entering/queued to enter, so the transport can decide whether to keep or restart its deploy/door timing. It does not order the receiver to move, drive, navigate, dock, or enter.

Active in standard YR: Conditional yes. The sender gate uses stock INI-driven `Passengers=` and `DeployTime=` fields, and standard YR includes passenger-capable units. The path only runs when a passenger-capable unit has an active deploy/door tracker and a first radio contact.

## 2. Key Offsets And Fields

| Evidence | Verified meaning | Notes |
|---|---:|---|
| `UnitTypeClass +0x5E0` | `Passengers=` integer | Parser uses string at `0x0081BBD4` and stores parsed integer at `0x00714B43..0x00714B50`. |
| Type data `+0x3C8/+0x3CC` | `DeployTime=` double | Parser uses string at `0x00843904`, default from existing type double, and stores via `FSTP` at `0x00714B77..0x00714B99`. |
| Object/Techno embedded tracker at `this+0x350` | deploy/door animation timing state | `0x004A51D0` tests inactive when bytes `+0x18` and `+0x19` are both zero. `0x004A5240` activates tracker and writes duration/current-frame data. |
| Radio vtable `+0x274` | `Transmit_Radio_ToFirst(message)` | Sender at `0x007366C5` uses this slot with `PUSH 0x11`. |
| Foot/Mission field `+0xB4` | queued mission | Receiver case checks `+0xB4 == 7` at `0x004D9228`. |

## 3. Live Sender

### Verified Sender

Function: `UnitClass::AI`  
Message: `0x11`  
Transmit helper: vtable slot `+0x274`, `Transmit_Radio_ToFirst`  
Assembly range: `0x0073668F..0x007366E6`

Relevant decompile:

```c
if ((((0 < *(int *)(param_1[0x1b1] + 0x5e0)) &&
      (cVar3 = FUN_004a51d0(), cVar3 == '\0')) &&
     (iVar7 = (**(code **)(*param_1 + 0x184))(), iVar7 != 0x10)) &&
    (iVar7 = (**(code **)(*param_1 + 0x274))(0x11), iVar7 != 1)) {
    FUN_004a5240(*(undefined4 *)(param_1[0x1b1] + 0x3c8),
                 *(undefined4 *)(param_1[0x1b1] + 0x3cc));
}
```

Assembly anchors:

- `0x0073668F`: loads unit type pointer from `ESI+0x6C4`.
- `0x00736695..0x0073669D`: reads type `+0x5E0` and skips if `Passengers <= 0`.
- `0x0073669F..0x007366A7`: calls `0x004A51D0` on `this+0x350`; send path requires tracker active.
- `0x007366B4..0x007366BA`: calls current mission virtual `+0x184`; skips if mission is `0x10`.
- `0x007366C1..0x007366C5`: `PUSH 0x11`; `CALL [vtable+0x274]`.
- `0x007366CB`: compares reply with `1`.
- `0x007366D0..0x007366E6`: if reply is not `1`, reads type `DeployTime` double from `+0x3C8/+0x3CC` and calls `0x004A5240` on `this+0x350`.

### Meaning From Sender Context

The sender is a passenger-capable unit asking its first radio contact whether that contact is still entering. A `ROGER`/`1` response suppresses restart of the sender's deploy/door timer. A non-`1` response restarts the sender's tracker using `DeployTime`.

This is a status poll. It does not pass a destination, target object, dock slot, cell coordinate, facing, or path request.

## 4. Receiver Handling

### FootClass Positive Response

Function: `FootClass::Receive_Radio` at `0x004D8FB0`  
Case: `0x11`  
Assembly range: `0x004D9219..0x004D9253`

Relevant decompile:

```c
case 0x11:
    iVar5 = (**(code **)(*param_1 + 0x184))();
    if ((iVar5 == 7) || (param_1[0x2d] == 7)) {
        TechnoClass__Receive_Radio(param_2,uVar4,param_4);
        return 1;
    }
    break;
```

Assembly anchors:

- `0x004D9219..0x004D921D`: calls current mission virtual `+0x184`.
- `0x004D9223`: compares current mission to `7`.
- `0x004D9228`: compares queued mission at `ESI+0xB4` to `7`.
- `0x004D9235..0x004D9242`: calls `TechnoClass::Receive_Radio` before returning success.
- `0x004D924A`: returns `EAX = 1`.
- If neither mission check matches, control falls through to base handling without a positive response.

### Class Fan-out

- `InfantryClass`: inherits `FootClass` handling for this path.
- `UnitClass`: no direct `0x11` case in `UnitClass::Receive_Radio`; default tail-calls `FootClass::Receive_Radio`, so unit receivers use the same mission-7 response.
- `AircraftClass`: no direct `0x11` case in `AircraftClass::Receive_Radio`; default tail-calls `FootClass::Receive_Radio`, so aircraft receivers use the same mission-7 response.
- `BuildingClass`: no direct `0x11` positive case; it falls through to `TechnoClass::Receive_Radio`, which does not make `0x11` a drive/nav/dock command.

## 5. Exhaustive Sender Scan

Search method: literal `PUSH 0x11` plus known radio transmit vtable calls.

Verified transmit slots scanned:

- `CALL [vtable+0x274]` / ToFirst: 36 contexts scanned. Only `0x007366C5` has preceding `PUSH 0x11`.
- `CALL [vtable+0x278]` / targeted transmit: 32 contexts scanned. No `0x11` sender found.
- `CALL [vtable+0x27C]` / payload transmit: 4 contexts scanned. Observed payload sends include `0x12`; no `0x11` sender found.
- `CALL [vtable+0x280]` / broadcast: 8 contexts scanned. No `0x11` sender found.

Conclusion: within the known live radio transmit helper slots, `UnitClass::AI @ 0x007366C1..0x007366C5` is the only verified `0x11` sender.

## 6. Active In Standard YR

Active in YR: Conditional yes.

Evidence:

- The sender gate reads `UnitTypeClass +0x5E0`, verified as `Passengers=` by the parser at `0x00714B43..0x00714B50`.
- The restart duration reads `+0x3C8/+0x3CC`, verified as `DeployTime=` by the parser at `0x00714B77..0x00714B99`.
- Retail standard YR `rulesmd.ini` includes passenger-capable units and `DeployTime=` values used by transport/passenger entry behavior.

Condition:

- A unit with `Passengers > 0` must have an active `this+0x350` deploy/door tracker and a first radio contact. The message is therefore live for transport entry state, not globally emitted by every passenger-capable unit every tick.

## 7. Rust Implementation Status

Current Rust has local boarding and reservation systems, not a general synchronous radio layer:

- `src/sim/passenger.rs` models `PassengerRole::Boarding { target_transport_id, phase }` and `BoardingPhase::{Approach, Entering}` with direct adjacency-based boarding.
- `src/sim/game_entity.rs` has `radio_contacts: Vec<u64>`, but current usage is not a faithful generic `RadioClass` protocol.
- `src/rules/object_type.rs` parses passenger capacity-related fields, but no matching `DeployTime` rule parse was found in the Rust scan.

## 8. Implementation Handoff

Suggested Rust behavior model:

1. Treat `0x11` as a transport/boarding status poll, not as a movement command.
2. A passenger/Foot receiver should answer success only while current or queued state is equivalent to YR `Mission_Enter`.
3. A passenger-capable transport with an active deploy/door timer should poll its first radio/contact; if the reply is not success, restart/maintain its door/deploy timer using parsed `DeployTime`.

Concrete test proposal:

- `transport_entry_poll_keeps_door_open_while_passenger_mission_enter`

## 9. Negative Facts / Do Not Do

- Do not label `0x11` as `DRIVE_TO`, `NAVIGATE`, `MOVE_TO_CELL`, or a refinery dock command.
- Do not make `BuildingClass::Receive_Radio` a positive `0x11` receiver without separate evidence; the verified positive response is in `FootClass`.
- Do not use `UnitType +0x5E0` as dock range in this context; parser evidence identifies it as `Passengers=`.
- Do not implement `0x11` as an order that sets destination, path, dock slot, or locomotor mission.
- Do not conflate this with `0x12`, which appears in separate payload transmit contexts and is the better candidate for move-to-cell style behavior.

## 10. Remaining Uncertainty

No material uncertainty remains for the scoped question of senders and meaning. The exact Westwood symbolic name is not present in the binary evidence reviewed, so `ARE_YOU_ENTERING` / `ENTERING_STATUS_POLL` is a behavior-derived name.

Deferred follow-up: exact visual frame consequences of `this+0x350` deploy/door tracker should be investigated separately if Rust implements transport door timing.

## 11. Stale Docs / Follow-up Wording

`RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` open question replacement:

> `0x11` has one verified radio sender: `UnitClass::AI @ 0x007366C1..0x007366C5`, via `Transmit_Radio_ToFirst`. It is a transport/passenger `ARE_YOU_ENTERING` status poll: sender gate is `Passengers > 0` and active `DeployTime` tracker; `FootClass` replies `1` only while current/queued mission is `Mission_Enter` (`7`). It is not a refinery drive/nav/dock instruction.

`FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` case `0x11` replacement:

> Replace `DRIVE_TO (Move-to-Cell dispatch / locomotor mission start)` with `ARE_YOU_ENTERING / transport passenger entry status poll`. Sender is `UnitClass::AI`, not `BuildingClass`. Receiver does not set movement/destination; it only returns `1` when current/queued mission is `7`, otherwise falls through.

`UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` offset wording:

> Rows calling `+0x5E0` dock range should be revisited. For the unit type parser evidence used by the `0x11` sender, `+0x5E0` is `Passengers=`. Any dock-range wording tied to this offset is stale unless another class/type-specific offset is proved separately.

## 12. Sources

- Ghidra decompile: `UnitClass::AI`, sender around `0x007366C1`.
- Ghidra assembly: `0x0073668F..0x007366E6`.
- Ghidra decompile: `FootClass::Receive_Radio @ 0x004D8FB0`.
- Ghidra assembly: `0x004D9219..0x004D9253`.
- Ghidra decompile/assembly: helper `0x004A51D0`, helper `0x004A5240`.
- Ghidra string and parser evidence: `Passengers` string `0x0081BBD4`, parser store `0x00714B43..0x00714B50`.
- Ghidra string and parser evidence: `DeployTime` string `0x00843904`, parser store `0x00714B77..0x00714B99`.
- Ghidra decompile: `UnitClass::Receive_Radio @ 0x00737430`.
- Ghidra decompile: `AircraftClass::Receive_Radio @ 0x004190B0`.
- Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Local Rust scan: `src/sim/passenger.rs`, `src/sim/game_entity.rs`, `src/rules/object_type.rs`.
