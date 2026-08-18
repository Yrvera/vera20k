# Refinery Dock 0x16 Bridge Verification - Ghidra Report

**Date:** 2026-05-24
**Binary:** `gamemd.exe`
**Investigation mode:** exhaustive-slice for `BuildingClass::Receive_Radio(0x0E) -> UnitClass::Receive_Radio(0x16) -> radio 0x15` and its relation to the `UnitClass::PerCellProcess` `GetDockCoord` gate.
**Scope:** Verify whether radio `0x16` physically moves/synchronizes a harvester from the accepted refinery cell `NW+(3,1)` to the stock `GetDockCoord` cell `NW+(2,1)` before radio `0x15`.
**Non-scope:** Full locomotor interpolation internals, full dump/departure mission, and non-refinery dock consumers.
**Active in YR:** Yes. Stock `[CMIN]`/`[HARV]` docking into `[GAREFN]`/`[NAREFN]` reaches this radio path through `DockUnload=yes`.
**Confidence:** High that `0x16` does not issue movement to `GetDockCoord`; High that `0x16` can cascade to `0x15` without a `GetDockCoord` cell comparison. Follow-up re-swarm reports on 2026-05-24 resolved the key tick-order uncertainty for implementation: stock docking is a staged MissionEnter/0x12/0x18/0x16 radio-timer handshake, not a physical NW+3 -> NW+2 bridge.

## 1. Bottom Line

The suspected "0x16 bridge" is not a physical move from accepted cell `NW+(3,1)` to `GetDockCoord` cell `NW+(2,1)`.

Live Ghidra verification shows:

1. `BuildingClass::Receive_Radio(0x0E)` sends `MOVE_TO_CELL(0x12)` to hardcoded building NW `+(3,1)`.
2. If that `0x12` reply is `0x14` / already-there, the building sends `0x18`, then `0x16`.
3. `UnitClass::Receive_Radio(0x16)` does not call `GetDockCoord`, does not call `Set_Destination`, and does not write a new cell.
4. `0x16` first calls the base radio chain, then sets/checks a `RateTimer`-backed field through the locomotor vtable `+0x4C`.
5. If the timer is already at `0x4000`, the locomotor reports not moving, the unit has a destination, the destination is a building, and the unit mission is `7`, `0x16` sends radio `0x15` directly to the destination building.
6. Separately, `UnitClass::PerCellProcess @ 0x00739EC0` can also send `0x15`, but that path compares current unit cell against destination building `GetDockCoord`.

Therefore, the previous implementation idea "make Rust move physically to `(12,11)` before any `0x15`" is too strong. gamemd has at least two `0x15` send paths:

| Source | Cell gate | Sends `0x15` when |
|---|---|---|
| `UnitClass::Receive_Radio(0x16)` | No `GetDockCoord` comparison | timer/locomotor/destination/mission checks pass after accepted `0x12` already-there |
| `UnitClass::PerCellProcess @ 0x00739EC0` | current cell == destination `GetDockCoord` cell | unit physically reaches the building dock coordinate |

## 2. Verified Standard Refinery `0x0E` Send Sequence

`BuildingClass::Receive_Radio @ 0x0043C2D0`, standard `DockUnload=yes` / `Weeder=yes` path:

- `0x0043CA71..0x0043CA8D`: calls building `Get_Cell_Packed`, adds `+3` to X and `+1` to Y.
- `0x0043CAA3..0x0043CAAE`: looks up the `CellClass*` for that hardcoded cell and writes it as the `0x12` payload.
- `0x0043CAB2..0x0043CAB8`: sends `MOVE_TO_CELL(0x12)` via vtable `+0x27C`.
- `0x0043CABE..0x0043CAC1`: requires reply `0x14` before continuing.
- `0x0043CAC7..0x0043CACE`: sends `0x18` to the harvester.
- `0x0043CAD4..0x0043CADB`: sends `0x16` to the harvester.
- `0x0043CAE1..0x0043CAF7`: if `0x16` reply is not `1`, plays the acceptance sound.

For stock refinery NW `(10,10)`, the `0x12` payload is cell `(13,11)`.

## 3. The Earlier `GetDockCoord` Touch In `0x0E`

`BuildingClass::Receive_Radio(0x0E)` also touches `GetDockCoord` before the hardcoded `0x12` payload:

- `0x0043C8E2..0x0043C8FA`: checks `DockUnload=yes` or `Weeder=yes`.
- `0x0043C911..0x0043C91B`: calls building vtable `+0xA8` / `GetDockCoord`.
- `0x0043C921..0x0043C927`: converts that coordinate to a `CellClass*`.
- `0x0043C92C..0x0043C93A`: compares against the requester's `+0x5A4`-related value and sets a stack sentinel if different.

This does not become the `0x12` move payload. The actual `0x12` payload is still generated later at `0x0043CA71..0x0043CAAE` from building NW `+(3,1)`.

Practical implication: the `GetDockCoord` touch inside `0x0E` is a side/admission state check, not the accepted movement destination.

## 4. Verified `UnitClass::Receive_Radio(0x16)` Receiver Logic

`UnitClass::Receive_Radio @ 0x00737430`, case `0x16`:

- `0x007376BA`: calls `FootClass::Receive_Radio(0x16)` first. That falls through to `TechnoClass::Receive_Radio(0x16)`, whose shared case with `7` and `9` transmits `0x18` back to the sender.
- `0x007376BF..0x007376C7`: checks byte `UnitClass+0x6AF`; if nonzero, skips the timer-set path.
- `0x007376CE..0x007376D9`: reads `RateTimer::Current` from `UnitClass+0x388` and compares the current word with `0x4000`.
- `0x007376E0..0x00737709`: if the current value is not `0x4000`, calls locomotor vtable `+0x4C` with argument `0x4000`, then returns `1`.
- `0x0073771B..0x0073773D`: otherwise checks locomotor vtable `+0x10` / `Is_Moving`.
- `0x0073773F..0x00737771`: if not moving, requires a destination, destination `WhatAmI() == 6` building, and unit mission `7`.
- `0x00737773..0x0073777A`: sends radio `0x15` to that destination building.
- `0x00737783`: returns `1`.

Negative evidence:

- No `GetDockCoord` call in case `0x16`.
- No `Set_Destination` call in case `0x16`.
- No `MOVE_TO_CELL(0x12)` send in case `0x16`.
- No write to unit `Location_X/Y/Z` in case `0x16`.

## 5. What Locomotor Vtable `+0x4C` Does In This Slice

The vtable target observed for the relevant call resolves to code at `0x004B0EF0`:

- `0x004B0EF0..0x004B0EF4`: reads the ILocomotion pointer and argument.
- `0x004B0F01`: reads linked object pointer from `[ILocomotion+0x8]`.
- `0x004B0F04`: adds `0x388`.
- `0x004B0F0A`: calls `RateTimer::Set @ 0x004C9220`.

This confirms the `0x16` path sets a `RateTimer` associated with the linked unit at offset `+0x388`. This report does not rename that field beyond the verified mechanism. The important parity fact is that this path updates a timer through the locomotor; it does not issue a new destination or cell movement.

## 6. Verified `UnitClass::PerCellProcess` `GetDockCoord` Gate

`UnitClass::PerCellProcess @ 0x00739EC0` contains a separate dock-arrival branch:

- It requires current mission `7` or `0x19`.
- It requires `FootClass::GetDestination()` to return a building.
- It calls the unit vtable `+0x48` to get current unit coordinates.
- It calls destination building vtable `+0xA8` / `GetDockCoord`.
- It converts both positions through the sign-correct lepton-to-cell shift.
- It compares cell X and cell Y.
- Only on equality does it call `FootClass::PerCellProcess(2)`, send radio `0x15`, then call locomotor vtable `+0x5C` / power off.

Key assembly confirmations:

- `0x0073A391..0x0073A3B1`: destination building vtable `+0xA8`.
- `0x0073A417..0x0073A437`: shifted cell X/Y compare.
- `0x0073A4F7..0x0073A507`: `FootClass::PerCellProcess(2)` then radio `0x15`.
- `0x0073A521..0x0073A52B`: locomotor vtable `+0x5C`.

For stock 4x3 GAREFN/NAREFN, the `GetDockCoord` cell is `NW+(2,1)`.

## 7. Resolution Of The Current Rust Question

The finding does **not** support replacing every accepted refinery target with `NW+(2,1)`.

It also does **not** support requiring physical movement to `NW+(2,1)` before every possible `0x15`, because `UnitClass::Receive_Radio(0x16)` can send `0x15` without a `GetDockCoord` comparison.

The correct implementation target is narrower:

1. Preserve the accepted `0x0E` / `0x12` target as `NW+(3,1)`.
2. Preserve the stock `GetDockCoord` cell as `NW+(2,1)` for the `PerCellProcess` arrival branch.
3. Model the `0x16` timing gate separately:
   - first `0x16` call may only set the `+0x388` timer through locomotor `+0x4C` and return;
   - a later/already-synchronized `0x16` can send `0x15` if idle, has destination, destination is building, and mission is `7`.
4. Do not fake the `0x16` effect by snapping only a Rust miner snapshot to `(12,11)`.
5. Do not introduce a forced physical step from `(13,11)` to `(12,11)`. Follow-up re-swarm evidence shows the miner can stop at accepted `(13,11)` with its refinery destination still active, and later/aligned `0x16` can send `0x15` from that stopped accepted-cell state.

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1: Does 0x16 move the unit to GetDockCoord?` No. It has no `GetDockCoord`, no `Set_Destination`, no location write, and no `0x12` send.
- `[RESOLVED] OQ-2: Can 0x16 itself send 0x15?` Yes, after the timer/idle/destination/building/mission gates pass.
- `[RESOLVED] OQ-3: Does PerCellProcess still have a GetDockCoord-gated 0x15 path?` Yes. It is a separate 0x15 source.
- `[RESOLVED] OQ-4: Is the accepted 0x12 cell still NW+(3,1)?` Yes. The hardcoded payload at `0x0043CA71..0x0043CAAE` uses building NW `+3,+1`.
- `[RESOLVED-BY-FOLLOWUP] OQ-5: In a stock harvester/refinery cycle, which 0x15 source fires first in every timing case?` The follow-up re-swarm verified the implementation-relevant ordering: `Mission_Enter` dispatch precedes same-tick locomotor/per-cell processing; accepted `0x12 == 1` sends only movement; a later 14-16 frame retry can get `0x12 == 0x14` and emit `0x18/0x16`; first ordinary `0x16` can synchronize rate/facing and return; later/aligned `0x16` can send `0x15` from stopped accepted-cell state. `PerCellProcess` still has a `GetDockCoord` equality branch and a contact-flag adjacent-building branch.

## 9. Implementation Handoff

Affected Rust surfaces:

- `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`
- `src/sim/miner/miner_dock_sequence.rs::phase_awaiting_accepted_cell`
- `src/sim/miner/miner_dock_sequence.rs::phase_linked`
- `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell`
- `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`
- `src/sim/miner/miner_tests.rs` docking tests around accepted cell, linked handoff, and pad occupancy

Acceptance scenarios to add before implementation:

1. First accepted-cell `0x16` equivalent sets/schedules the `0x4000` timing state but does not necessarily start unload in the same tick when the timer was not already synchronized.
2. Already-synchronized, idle, mission-7 miner with building destination can trigger the `0x15` handoff from the `0x16` path without a `GetDockCoord` equality check.
3. `PerCellProcess` path still triggers `0x15` only when current cell equals stock `GetDockCoord` cell.
4. Rust does not silently desynchronize snapshot cell from physical entity cell unless that corresponds to a verified gamemd state field split.

Do not do:

- Do not change accepted `0x0E` target from `NW+(3,1)` to `NW+(2,1)`.
- Do not assume `0x16` is a physical pad move.
- Do not require `NW+(2,1)` before every `0x15`; follow-up evidence proves the staged `0x16` path can unload from stopped accepted-cell state.
- Do not keep a vague `refinery_pad_cell` name for both accepted target and `GetDockCoord` cell; those are different reference points.

## 10. Sources

- Ghidra `decompile_function ram:00739ec0`
- Ghidra `batch_decompile UnitClass__Receive_Radio`
- Ghidra `batch_decompile BuildingClass__Receive_Radio`
- Ghidra `batch_decompile FootClass__Mission_Enter`
- Ghidra `batch_decompile FootClass__Receive_Radio`
- Ghidra `batch_decompile TechnoClass__Receive_Radio`
- Ghidra `get_assembly_context 0x007376BA,0x007376D4,0x00737709,0x00737738,0x0073777A`
- Ghidra `get_assembly_context 0x0073A391,0x0073A417,0x0073A503,0x0073A527`
- Ghidra `disassemble_function ram:0043c2d0`
- Existing docs read during verification:
  - `docs/research/RADIO_0x16_RECEIVER_UNITCLASS_CASE_16_GHIDRA_REPORT.md`
  - `docs/research/RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md`
  - `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`
  - `docs/research/UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_GETDOCKCOORD_GHIDRA_REPORT.md`
- Follow-up reports resolving OQ-5:
  - `docs/research/FOOTCLASS_MISSION_ENTER_0X0E_REPEAT_TIMING_GHIDRA_REPORT.md`
  - `docs/research/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
  - `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`
  - `docs/research/BUILDING_RECEIVE_RADIO_0E_GETDOCKCOORD_SIDE_CHECK_GHIDRA_REPORT.md`
  - `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`
