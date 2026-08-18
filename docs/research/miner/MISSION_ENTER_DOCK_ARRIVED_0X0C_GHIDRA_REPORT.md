# Mission Enter Dock Arrived 0x0C - Ghidra Research Report

**Address(es):** `0x004D9290` (`FootClass__Mission_Enter`), `0x0041AA80` (Ghidra label `UnitClass__EnterBuildingOrDock`, identity corrected below), `0x00741970` (`TechnoClass__Set_Destination`), `0x00739EC0` (`UnitClass__PerCellProcess`), `0x0043C2D0` (`BuildingClass__Receive_Radio`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Whether a stock YR harvester sends radio `0x0C` (`DOCK_ARRIVED`) during inbound refinery docking, and the exact sender-side order around approach, arrival, `0x0E`, `0x16`, and `0x15`.  
**Non-Scope:** Full receiver case `0x08`, exit `0x19`, full unload loop, full aircraft docking, and suspicious non-refinery radio cases.  
**Confidence:** High for the negative `0x0C` answer and the sender-side order.  
**Active in YR:** Yes for stock HARV/CMIN to GAREFN/NAREFN. `0x0041AA80` is active in YR, but not for stock harvester-refinery docking.

## 1. Overview

The inbound stock refinery dock path does **not** use harvester -> refinery radio `0x0C`. The harvester approaches by sending `0x0E` (`CAN_DOCK`) from Mission_Enter / destination setup, receives the building's `0x13 -> 0x12 -> 0x18 -> 0x16` reply burst, and on dock-pad arrival sends `0x15` (`TIMING_SYNC_BACK` / dock handoff) to the refinery.

The plan's `UnitClass::EnterBuildingOrDock @ 0x0041AA80` target is a label trap. Static vtable bytes put `0x0041AA80` in the AircraftClass destination slot. UnitClass vtable `+0x480` instead points at `0x00741970` (`TechnoClass__Set_Destination`), which is the stock harvester destination path.

## 2. Verified Answer

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| A stock harvester does not send `0x0C` during inbound refinery docking. | Full decompile of `FootClass__Mission_Enter @ 0x004D9290`; radio inventory has `0x0E` at `0x004D92B5..0x004D92B9` and failure `0x03` at `0x004D92D0..0x004D92D4`, not `0x0C`. | High | Yes |
| Dock-pad arrival is `UnitClass__PerCellProcess @ 0x00739EC0` sending `0x15`, not `0x0C`. | `0x0073A4F7` calls `FootClass__PerCellProcess(2)`, `0x0073A503..0x0073A507` sends `0x15` via vtable `+0x274`, then locomotor `+0x674` is called through vtable `+0x5C`. | High | Yes |
| The refinery starts the dump from radio `0x15`, not `0x0C`. | `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15`: `Type+0x16B3` (`DockUnload=yes`) gates the branch; `0x0043C79A PUSH 0`, `0x0043C79C PUSH 0x10`, `0x0043C7A0 CALL [EDX+0x1E8]` queues sender mission `0x10`. | High | Yes, `rulesmd.ini:[GAREFN] DockUnload=yes` line 11726 and `[NAREFN] DockUnload=yes` line 12519 |
| Building case `0x0C` exists but is not the stock refinery arrival handoff. | `BuildingClass__Receive_Radio` case `0x0C` checks receiver mission `0x13`, may set mission `5`, and can clear/create anim slots for `Type+0x16B9`; no verified harvester sender path sends it during stock refinery arrival. | High for branch content | Conditional: present in YR binary, not stock HARV/CMIN refinery arrival |
| `0x0041AA80` is not the stock harvester helper despite its Ghidra label. | `read_memory 0x007E2724` returns `80 aa 41 00` -> `0x0041AA80` in AircraftClass vtable; `read_memory 0x007F60F0` returns `70 19 74 00` -> `0x00741970` for UnitClass vtable `+0x480`. | High | `0x0041AA80`: Yes for aircraft, No for standard harvester-refinery docking |

## 3. Sender-Side Radio Order

1. Mission 7 / Mission_Enter approach tick: `FootClass__Mission_Enter @ 0x004D9290` gets or filters a destination building and sends directed radio `0x0E` through vtable `+0x278`. Evidence: `0x004D92B5 PUSH 0xE`, `0x004D92B9 CALL [EDX+0x278]`. Active in YR: Yes.
2. If `0x0E` is not `ROGER(1)` and the unit's linked/docked byte at `+0x418` is clear, Mission_Enter sends `0x03` through vtable `+0x274` and clears destination. Evidence: `0x004D92BF CMP EAX,0x1`; `0x004D92D0 PUSH 0x3`, `0x004D92D4 CALL [EAX+0x274]`. Active in YR: Yes, failure path.
3. `BuildingClass__Receive_Radio` case `0x0E` accepts and emits the reply burst: `0x13` probe, `0x12` move-to-cell payload, `0x18` enter-dock, then `0x16` timing sync. Evidence: decompile `0x0043C2D0`; prior `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`. Active in YR: Yes for `DockUnload=yes`.
4. `UnitClass__Receive_Radio @ 0x00737430` case `0x16` may immediately cascade `0x15` back if the unit is already in mission `7`. Evidence: `0x0073776E CMP EAX,0x7`; `0x00737776 PUSH 0x15`, `0x0073777A CALL [EDX+0x278]`. Active in YR: Yes, but this is timing-sync cascade, not `0x0C`.
5. On actual pad-cell arrival, `UnitClass__PerCellProcess @ 0x00739EC0` sends `0x15` to the existing contact via vtable `+0x274`, then stops/powers off the locomotor. Evidence: `0x0073A4F7 PUSH 0x2`, `0x0073A503 PUSH 0x15`, `0x0073A507 CALL [EDX+0x274]`, then `+0x674` locomotor call at `+0x5C`. Active in YR: Yes.
6. The refinery receives `0x15` and queues sender mission `0x10`; this enters `UnitClass__Mission_Deploy_Building` for the unload FSM. Evidence: `BuildingClass__Receive_Radio @ 0x0043C2D0`, case `0x15`, assembly context `0x0043C788..0x0043C7A0`. Active in YR: Yes.

No `0x0C` appears in this sender order.

## 4. Key Offsets And Slots

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Radio vtable `+0x278` | Directed radio send. | `0x004D92B9` sends `0x0E`; `0x00741DDA` sends `0x0E`; `0x0073777A` sends `0x15` in the `0x16` cascade. | Yes |
| Radio vtable `+0x274` | Send to first/current radio contact. | `0x004D92D4` sends failure `0x03`; `0x0073A507` sends arrival `0x15`. | Yes |
| Mission setter vtable `+0x1E8` | Receiver-side mission transition. | `0x0043C7A0` queues sender mission `0x10` after refinery receives `0x15`. | Yes |
| BuildingType `+0x16B3` | `DockUnload=yes`. | `BuildingClass__Receive_Radio` case `0x15`; INI `rulesmd.ini:[GAREFN]` line 11726, `[NAREFN]` line 12519. | Yes |
| Unit `+0x5A4` | Current dock/radio-linked building used by pad-arrival compare. | `UnitClass__PerCellProcess @ 0x00739EC0` compares destination to `[EBP+0x5A4]` before sending `0x15`. | Yes |
| Unit `+0x674` | Locomotor COM pointer used after pad-arrival `0x15`. | `0x0073A50D` load/null-check, then vtable `+0x5C`. | Yes |

## 5. INI Activation

| INI path | Evidence | Effect | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN] Dock=NAREFN,GAREFN` | line 7361 | CMIN can target Allied/Soviet refineries. | Yes |
| `rulesmd.ini:[CMIN] Harvester=yes` | line 7364 | CMIN uses harvester return/dock logic. | Yes |
| `rulesmd.ini:[HARV] Dock=NAREFN,GAREFN` | line 8225 | HARV can target Allied/Soviet refineries. | Yes |
| `rulesmd.ini:[HARV] Harvester=yes` | line 8228 | HARV uses harvester return/dock logic. | Yes |
| `rulesmd.ini:[GAREFN] DockUnload=yes` | line 11726 | Selects receiver case `0x15` DockUnload handoff. | Yes |
| `rulesmd.ini:[NAREFN] DockUnload=yes` | line 12519 | Same. | Yes |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass__Mission_Enter @ 0x004D9290` radio sends | verified | decompile plus contexts `0x004D92B5..0x004D92D4` | none for `0x0C` question |
| `0x0041AA80` identity | verified | `read_memory 0x007E2724` and `0x007F60F0`; decompile `0x0041AA80` | full aircraft dock behavior out-of-scope |
| Real UnitClass/harvester destination path `0x00741970` | touched-not-exhausted | decompile `0x00741970`; context `0x00741DDA` | broad Set_Destination semantics out-of-scope |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` cases `0x0C`, `0x0E`, `0x15` | verified for this slice | decompile `0x0043C2D0`; context `0x0043C788..0x0043C7A0` | full case table out-of-scope |
| `UnitClass__PerCellProcess @ 0x00739EC0` dock-arrival handoff | verified | decompile and context `0x0073A4F7..0x0073A52B` | full per-cell function out-of-scope |
| `UnitClass__Receive_Radio @ 0x00737430` case `0x16` cascade | touched-not-exhausted | context `0x00737776..0x0073777A` | full case `0x16` body already covered by prior doc |
| Stock INI activation | verified | `rulesmd.ini` line hits for CMIN/HARV/GAREFN/NAREFN | none |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - Does a harvester send `0x0C DOCK_ARRIVED` during inbound refinery docking? No. Verified sender functions in this slice send `0x0E`, optional `0x03`, and arrival `0x15`, with no `0x0C` in the stock path. Evidence: `0x004D9290`, `0x00741970`, `0x00739EC0`, `0x0043C2D0`.

[RESOLVED] OQ-2 - If not `0x0C`, what handles arrival? `UnitClass__PerCellProcess @ 0x00739EC0` detects the dock-cell condition and sends `0x15` through the current radio contact. Evidence: `0x0073A4F7..0x0073A507`.

[RESOLVED] OQ-3 - What does the refinery do after `0x15`? `BuildingClass__Receive_Radio` case `0x15` gates on `DockUnload=yes` and queues sender mission `0x10`. Evidence: `0x0043C788..0x0043C7A0`.

[RESOLVED] OQ-4 - Is `0x0041AA80` the harvester dock sender? No. It is bound from AircraftClass vtable bytes at `0x007E2724`; UnitClass vtable `+0x480` points at `0x00741970`. Evidence: static `read_memory` results.

[DEFERRED] OQ-5 - Exact stock non-refinery meaning of BuildingClass case `0x0C`. Category: out-of-scope. It is present in the binary and conditionally active, but this slot only needed to prove it is not the stock refinery arrival handoff.

## Sources

- Ghidra `decompile_function 004d9290`; `get_assembly_context 004d92b5,004d92d0`.
- Ghidra `decompile_function 0041aa80`; `read_memory 007E2724,16`; `read_memory 007F60F0,8`.
- Ghidra `decompile_function 00741970`; `get_assembly_context 00741dda`.
- Ghidra `decompile_function 00739ec0`; `get_assembly_context 0073a503`.
- Ghidra `decompile_function 0043c2d0`; `get_assembly_context 0043c788`.
- Ghidra `get_assembly_context 00737776,0073777a`.
- `ini/rulesmd.ini` line hits for `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`.
- Prior docs: `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `UNITCLASS_ENTERBUILDINGORDOCK_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`, `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`.
