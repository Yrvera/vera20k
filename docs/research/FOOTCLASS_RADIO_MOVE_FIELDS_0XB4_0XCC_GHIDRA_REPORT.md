# FootClass Radio Move Fields +0xB4/+0xCC - Ghidra Research Report

**Address(es):** `0x004D8FB0` (`FootClass::Receive_Radio`), `0x005B35E0`, `0x005B3040`, `0x005B3060`, `0x0043C2D0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** FootClass radio cases `0x11` and `0x12`, with immediate mission/timer helpers and refinery dock senders needed to resolve `this+0xB4` and `this+0xCC`.
**Non-Scope:** Full radio protocol, full `TechnoClass::Set_Destination`, full locomotor movement process, and non-refinery senders of `0x11`.
**Confidence:** High for field identity and write order; Medium for visible impact because no live runtime watchpoint was used.
**Active in YR:** Yes / Conditional. Case `0x12` is active in standard YR refinery docking. Case `0x11` is active code, but this slice found no standard refinery dock sender.

## 1. Overview

The disputed `FootClass+0xB4` field is not a team id in these radio cases. It is `MissionClass::QueuedMission`, initialized to `-1`, written by `MissionClass::Queue_Mission`, and used as the fallback mission when `CurrentMission` is `-1`.

The disputed `FootClass+0xCC` write in case `0x12` is not a standalone chrono miner timestamp. Case `0x12` writes the MissionClass dispatch timer triplet at `+0xC8..+0xD0`: `+0xC8 = g_CurrentFrameCounter`, `+0xCC = target_coord_y` from the local coordinate buffer when the payload target is non-null, and `+0xD0 = 0`. `MissionClass::Mission_Dispatch` gates on `+0xC8` and `+0xD0`; this slice found no timing read of `+0xCC` on the checked dispatch path.

## 2. Class Layout / Key Offsets

| Offset | Type | Role | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0xAC` | `int` | `CurrentMission` | `MissionClass::GetCurrentMission @ 0x005B3040` reads `+0xAC`, falls back if `-1`; `Mission_Dispatch @ 0x005B3060` switches on `param_1[0x2B]` | Yes |
| `+0xB4` | `int` | `QueuedMission` | Constructor `0x005B2DA0` sets `param_1[0x2D] = -1`; `Queue_Mission @ 0x005B35E0` writes `param_1[0x2D] = mission` | Yes |
| `+0xB8` | byte | queued-mission force/aux flag | `Queue_Mission @ 0x005B35E0` clears `*(byte *)(param_1 + 0x2E)` when queue changes | Yes |
| `+0xC8` | `int` | Mission dispatch timer start frame | `Mission_Dispatch @ 0x005B3060` compares `g_CurrentFrameCounter - param_1[0x32]` to `param_1[0x34]`; case `0x12` writes current frame at `0x004D91F1..0x004D9203` | Yes |
| `+0xCC` | `int` | Middle dword of timer storage; case `0x12` fills it from target coord Y local | Case `0x12`: target `vtable+0x48` writes coord buffer at stack `+0x1C`; `MOV EDX,[ESP+0x20]` at `0x004D91F6`, then `MOV [ESI+4],EDX` after `ESI += 0xC8` | Yes |
| `+0xD0` | `int` | Mission dispatch timer duration | `Mission_Dispatch @ 0x005B3060` reads `param_1[0x34]`; case `0x12` writes zero at `0x004D920D` | Yes |

## 3. Core Logic

### Case `0x11`

`FootClass::Receive_Radio @ 0x004D8FB0` case `0x11` reads the effective current mission through vtable `+0x184` and separately checks `QueuedMission` at `+0xB4`.

- If effective mission is `7` or `QueuedMission == 7`, it delegates to `TechnoClass::Receive_Radio` and returns `1`.
- Otherwise it falls through to the common tail, which delegates to `TechnoClass::Receive_Radio` and returns that result.
- There is no write to `+0xB4`, `+0xC8`, `+0xCC`, or `+0xD0` in this case.

**Active in YR:** Conditional. The handler is live for FootClass-derived objects, but `BuildingClass::Receive_Radio @ 0x0043C2D0` standard refinery case `0x0E` sends `0x13 -> 0x12 -> 0x18 -> 0x16`, not `0x11`. Existing sender evidence in the prior full switch report points to carryall/non-refinery context, not the standard refinery dock path.

### Case `0x12`

Case `0x12` is the active standard refinery `MOVE_TO_CELL` assignment.

1. If `*payload` is non-null, call target `vtable+0x48` into a local 3-int coord buffer at stack `+0x1C`.
2. Convert target X/Y leptons to cell X/Y using `(value + ((value >> 31) & 0xFF)) >> 8`.
3. Call self `vtable+0x1B8` to get current cell.
4. If current cell equals target cell, return `0x14` without changing destination or timer fields.
5. If current effective mission is `5` and `QueuedMission == -1`, queue mission `2`.
6. If `QueuedMission == 7` and `CanResumeMission()` returns true, call `ResumeMission()`.
7. Call `Set_Destination(*payload, 1)` through vtable `+0x480`.
8. Write `+0xC8 = g_CurrentFrameCounter`, `+0xCC = [stack+0x20]`, `+0xD0 = 0`, then return `1`.

For the standard non-null payload path, `[stack+0x20]` is the second dword of the target coord buffer, i.e. target Y in leptons. For a null `*payload`, the field still gets written from the same stack slot, but the slot is not initialized by the case-local target coordinate call. That null-payload edge was not found in the standard refinery sender path.

**Active in YR:** Yes. `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` sends `0x12` to the harvester during standard `DockUnload=yes` refinery admission.

## 4. INI Keys

| Key | Location | Default / stock value | Effect on this slice | Active in YR |
|---|---|---|---|---|
| `Dock=` | `rulesmd.ini` `[CMIN]`, `[CMON]`, `[HARV]`, `[HORV]` | `NAREFN,GAREFN` | Makes standard miners eligible for refinery radio docking | Yes |
| `Harvester=yes` | same unit sections | yes | Puts unit on harvester return/dock path | Yes |
| `Teleporter=yes` | `rulesmd.ini` `[CMIN]` line 7396 | yes | Makes Chrono Miner use teleport locomotor, but does not alter case `0x12` field writes | Yes |
| `Locomotor=` | `rulesmd.ini` `[CMIN]` line 7398 | `{4A582747-9839-11d1-B709-00A024DDAFD1}` | Teleport locomotor for chrono miner; `Set_Destination` handles locomotor consequences outside this slice | Yes |
| `DockUnload=yes` / `Refinery=yes` | `rulesmd.ini` `[GAREFN]`, `[NAREFN]` | yes | Selects building receiver path that sends `0x12` | Yes |
| `NumberOfDocks=1` | `rulesmd.ini` `[GAREFN]`, `[NAREFN]` | 1 | Single refinery dock in stock YR | Yes |
| `ChronoHarvTooFarDistance=50` | `rulesmd.ini` `[General]` line 294 | 50 cells | Determines upstream harvester return branch; not read by FootClass case `0x12` | Yes |

## 5. Integration Points

`MissionClass::Queue_Mission @ 0x005B35E0` is the direct writer of `+0xB4`. It writes a new queued mission only when the requested mission is not `-1` and is not redundant with the current/queued state, then clears the byte at `+0xB8`.

`MissionClass::GetCurrentMission @ 0x005B3040` reads `+0xAC`; if it is `-1`, it returns `+0xB4`. This explains why case `0x11` checks both the effective mission and raw `+0xB4`: queued Mission 7 matters before it becomes current.

`MissionClass::Mission_Dispatch @ 0x005B3060` reads `+0xC8` and `+0xD0` before dispatch. If `+0xC8 != -1` and `g_CurrentFrameCounter - +0xC8 < +0xD0`, dispatch waits. With case `0x12` writing `+0xD0 = 0`, the next dispatch is not delayed by this timer.

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` is the standard refinery sender. For refinery/weeder buildings it computes the queue cell, stores it in the payload, and sends `0x12` to the harvester as part of the same synchronous admission burst.

`UnitClass::Receive_Radio @ 0x00737430` case `0x0E` also sends `0x12` after a `0x13` gate for unit-side docking/loading contexts. This confirms case `0x12` is shared radio machinery, not chrono-miner-specific.

## 6. Current Rust Implementation Status

Current Rust does not model `MissionClass::QueuedMission` or the `+0xC8..+0xD0` mission dispatch timer triplet as such. Miner behavior is represented by explicit Rust states and dock/contact structures:

- `src/sim/miner/mod.rs:241` defines `Miner` with `state`, `reserved_refinery`, `dock_queued`, `dock_phase`, and timers for harvest/unload/deposit cooldown.
- `src/sim/miner/miner_dock.rs:21` defines `RefineryDockContacts` with `contacts`, `waiting_retry_queue`, and `on_pad`.
- `src/rules/ruleset.rs:1697` resolves `Dock=` eligibility through `harvester_can_dock_at`.
- `src/rules/object_type.rs:426` parses `Teleporter=`, and `src/sim/movement/teleport_movement.rs:96` treats harvesters specially for chrono delay.

This is architecturally different from gamemd's radio/mission timer storage. For parity, the visible requirement from this slice is that a refinery `0x12` assignment resets the unit's mission dispatch delay to zero and sets the destination immediately; there is no evidence that `+0xCC` itself controls chrono miner dock timing.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::Receive_Radio` case `0x11` | verified | `0x004D8FB0`; branch at `0x004D9219`; prior disassembly notes `0x004D9228` checks `+0xB4 == 7` | Non-refinery sender inventory deferred |
| `FootClass::Receive_Radio` case `0x12` | verified | `0x004D9139..0x004D9210`; bytes decoded for coord local and timer write | Runtime watchpoint not used |
| `+0xB4` identity | verified | `MissionClass::Constructor @ 0x005B2DA0`; `GetCurrentMission @ 0x005B3040`; `Queue_Mission @ 0x005B35E0` | none |
| `+0xCC` source in standard non-null case `0x12` | verified | target coord buffer at stack `+0x1C`; `MOV EDX,[ESP+0x20] @ 0x004D91F6`; write to `[ESI+4]` after `ESI += 0xC8` | Null-payload value deferred as out-of-standard-path |
| Mission dispatch use of `+0xC8/+0xD0` | verified | `MissionClass::Mission_Dispatch @ 0x005B3060` | Other readers of `+0xCC` outside dispatch not exhaustively scanned |
| Standard refinery sender of `0x12` | verified | `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E`; prior doc `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH` lines 204-208 | none |
| Standard refinery sender of `0x11` | verified-negative for checked path | `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` emits `0x13 -> 0x12 -> 0x18 -> 0x16`; no `0x11` in this path | Broad global sender search deferred |
| Chrono miner INI activation | verified | `rulesmd.ini` `[CMIN]`: `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Teleporter=yes`, teleport locomotor | none |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What is `this+0xB4` in cases `0x11`/`0x12`? It is `MissionClass::QueuedMission`, not a team id. Evidence: constructor `0x005B2DA0`, getter `0x005B3040`, writer `0x005B35E0`.

[RESOLVED] OQ-2 - What is the `iStack_10` / `this+0xCC` write in case `0x12`? In the standard non-null payload path it is target coord Y from the local coordinate buffer, then stored into the middle dword of `+0xC8..+0xD0`; dispatch timing uses `+0xC8` and `+0xD0`. Evidence: `0x004D9144..0x004D914E`, `0x004D91F1..0x004D920D`, `0x005B3060`.

[RESOLVED] OQ-3 - Does this affect chrono miner dock timing visibly in standard YR? Yes for `+0xC8/+0xD0` reset ordering because standard refinery `0x12` makes the next mission dispatch eligible immediately after setting destination. No evidence that `+0xCC` itself affects visible chrono miner dock timing in the checked path. Evidence: `0x0043C2D0` sender, `0x004D91E1..0x004D920D` write order, `0x005B3060` timer gate.

[DEFERRED] OQ-4 - Are there non-refinery active YR senders of `0x11` beyond prior carryall evidence? Deferred; out-of-scope for this refinery dock field slice.

[DEFERRED] OQ-5 - What exact value lands in `+0xCC` if case `0x12` is invoked with `*payload == NULL`? Deferred; not reached by standard refinery case `0x0E`, and resolving it requires runtime or a full sender inventory.

## Sources

- Ghidra read-only decompile: `FootClass::Receive_Radio @ 0x004D8FB0`
- Ghidra read-only byte decode: `0x004D9139..0x004D9210`
- Ghidra read-only decompile: `MissionClass::Constructor @ 0x005B2DA0`
- Ghidra read-only decompile: `MissionClass::GetCurrentMission @ 0x005B3040`
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`
- Ghidra read-only decompile: `MissionClass::Queue_Mission @ 0x005B35E0`
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra read-only decompile: `UnitClass::Receive_Radio @ 0x00737430`
- INI: `ini/rulesmd.ini` `[CMIN]`, `[CMON]`, `[HARV]`, `[HORV]`, `[GAREFN]`, `[NAREFN]`, `[General]`
- Prior docs: `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`
