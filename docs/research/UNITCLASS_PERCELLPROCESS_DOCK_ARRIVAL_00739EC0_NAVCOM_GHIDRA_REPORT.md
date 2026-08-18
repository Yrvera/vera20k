# UnitClass::PerCellProcess Dock Arrival 0x00739EC0 NavCom - Ghidra Report

**Address(es):** `0x00739EC0` primary; hot branch `0x0073A31F..0x0073A52B`; fallback/cascade check `0x0073A558..0x0073A5C8`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** standard YR CMIN/HARV physical dock-pad arrival into GAREFN/NAREFN: mission/cell gates, dock-link reads/writes, radio `0x15` emission, locomotor slot `+0x5C` ordering, and whether radio `0x10` or a fallback path fires on the successful arrival.
**Non-Scope:** full harvest search, full dump economics, post-unload exit track, non-refinery enter systems, and semantic naming of every locomotor vtable slot.
**Confidence:** High for this slice.
**Active in YR:** Yes for standard CMIN/HARV -> GAREFN/NAREFN dock arrival. `ini/rulesmd.ini:[CMIN]` and `[HARV]` set `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]` and `[NAREFN]` set `DockUnload=yes` and `Refinery=yes`.

## 1. Overview

`UnitClass::PerCellProcess @ 0x00739EC0` sends the successful refinery pad-arrival handoff as radio `0x15`, not radio `0x10`. The ordering on the taken branch is: mission/destination/cell/locomotor gates, optional `+0x5A4` repair-pad fill, `destination == unit+0x5A4`, `FootClass::PerCellProcess(2)`, radio `0x15` through vtable `+0x274`, then locomotor vtable `+0x5C`.

For stock DockUnload refineries, the receiver is `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15`, which queues mission `0x10` on the sender. That `0x10` is a mission id, not a radio message.

## 2. Key Offsets / Slots

| Field / slot | Behavior in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit current mission via vtable `+0x184` | Hot branch accepts mission `7` or `0x19` only. | `0x0073A31F..0x0073A33D` | Yes |
| Destination object | Must be non-null and `WhatAmI()==6` (building). | `0x0073A343..0x0073A359` | Yes |
| Cell-center compare | Unit coordinate and building dock coordinate are normalized with `cell * 0x100 + 0x80`, then X/Y cells must match. | `0x0073A36F..0x0073A437` | Yes |
| Unit `+0x674` | Locomotor pointer; checked before piggyback query and reloaded before slot `+0x5C`. | `0x0073A43D`, `0x0073A50D`, `0x0073A521` | Yes |
| IPiggyback CLSID | Queried from locomotor and compared with `CLSID_WalkLocomotion`. | `0x0073A44F..0x0073A4CB` | Yes; this is the drive-in/walk-piggyback state used by CMIN dock approach |
| BuildingType `+0x16A9` | Optional repair-pad gate for filling `unit+0x5A4` if empty. | `0x0073A4D5..0x0073A4E9` | Conditional; not stock GAREFN/NAREFN |
| Unit `+0x5A4` | Dock/NavCom link compared against arrived building; only conditional write here is the `+0x16A9` path. | `0x0073A4DF`, `0x0073A4E9`, `0x0073A4EF` | Yes as compare/read; conditional write no for stock refinery |
| Unit vtable `+0x274` | Sends radio `0x15` to first/current radio contact. | `0x0073A503..0x0073A507`; `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0` | Yes |
| Locomotor vtable `+0x5C` | Called after radio `0x15`; exact global semantic deferred, ordering verified. | `0x0073A521..0x0073A52B` | Yes |
| Unit `+0x418` | Read by a later fallback/cascade branch; not written by the hot arrival block. | `0x0073A558`; writer is radio `0x18` at `0x006F4B72` | Yes, but not the primary successful-return path after hot branch returns |
| BuildingType `+0x16B3` | `DockUnload=yes`; receiver case `0x15` queues sender mission `0x10`. | `BuildingClass::Receive_Radio @ 0x0043C2D0`; `rulesmd.ini:11726`, `12519` | Yes |

## 3. Core Logic

### 3.1 Timing / gates

The pad-arrival block only runs during the per-cell processing call with reason `2`, after the unit is already in a cell-arrival context. Inside that call, the branch requires current mission `7` or `0x19`, a non-null destination, destination `WhatAmI()==6`, X/Y equality between the unit's current cell and the destination building dock coordinate, a non-null locomotor at `+0x674`, and an IPiggyback CLSID equal to `CLSID_WalkLocomotion`.

**Active in YR:** Yes. `UnitClass::Mission_Harvest @ 0x0073E5E0` state 3 queues mission `7`, and the stock refinery admission path leads CMIN/HARV to the building dock cell.

### 3.2 Hot branch ordering

On the successful stock arrival branch:

1. The function optionally fills `unit+0x5A4` only if the destination building type has `+0x16A9` and the field is null.
2. It requires `destination == unit+0x5A4`.
3. It calls `FootClass::PerCellProcess(2)` at `0x0073A4F7..0x0073A4FB`.
4. It sends radio `0x15` through vtable `+0x274` at `0x0073A503..0x0073A507`.
5. It reloads `unit+0x674`, asserts if null, and calls locomotor slot `+0x5C` at `0x0073A521..0x0073A52B`.
6. It releases the piggyback interface if present and returns.

**Active in YR:** Yes. The optional `+0x16A9` fill is conditional/non-stock for GAREFN/NAREFN, but the compare, `0x15` send, and locomotor ordering are active for standard DockUnload refineries.

### 3.3 Receiver handoff: radio `0x15` -> mission `0x10`

`RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0` forwards message `0x15` synchronously through `Transmit_Radio_Impl @ 0x0065A970` to the target receiver. `0x15` is not `HELLO(0x02)` or `BREAK(0x03)`, so the radio helper performs generic target `Receive_Radio` dispatch and does not mutate contact slots.

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` checks receiver type flags. For stock `DockUnload=yes` (`+0x16B3`) it calls the sender mission queue slot `+0x1E8` with mission `0x10`, queued flag `0`, and returns `1`. `MissionClass::Queue_Mission @ 0x005B35E0` for `(0x10,0)` writes mission queue fields (`+0xB4`, `+0xB8`) if needed and does not commence immediately through the `param_3 != 0` block.

**Active in YR:** Yes. `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`; CMIN/HARV can target them via `Dock=NAREFN,GAREFN`.

### 3.4 Radio `0x10` verdict

No radio `0x10` fires from the successful `0x00739EC0` dock-arrival branch. The only radio emitted by the branch is `0x15`. The later `0x10` is the mission id queued by the building after receiving `0x15`.

`BuildingClass::Receive_Radio` does contain a receiver-side case `0x10` for `Refinery=yes` / `UnitRepair=yes` / `Weeder=yes`, but the reviewed arrival branch does not send it. The prior sender trace also scanned the major harvester/refinery radio senders and found no standard YR sender for radio `0x10`.

**Active in YR:** No for radio `0x10` on standard CMIN -> GAREFN/NAREFN arrival. Conditional receiver code exists, but no standard sender is active on this path.

### 3.5 Fallback / cascade branch

If the primary arrival block is not taken or falls through, a later branch reads `unit+0x418`, requires destination building and mission `7`, probes the cell one row above the unit, and can send directed radio `0x15` through vtable `+0x278` at `0x0073A5C3..0x0073A5C8`. It still sends `0x15`, not `0x10`.

For the successful standard CMIN/HARV -> GAREFN/NAREFN pad arrival where `destination == unit+0x5A4`, the primary branch returns immediately after locomotor `+0x5C`, so this fallback/cascade branch does not fire in the same call.

**Active in YR:** Conditional. The code is live, but it is not the successful stock refinery pad-arrival handoff when the primary equality check passes.

## 4. INI Activation

| INI path | Relevant values | Effect | Active in YR |
|---|---|---|---|
| `ini/rulesmd.ini:[CMIN]` | `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Teleporter=yes` at lines `7361`, `7364`, `7396` | Chrono Miner uses the harvester/refinery dock chain. | Yes |
| `ini/rulesmd.ini:[HARV]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` at lines `8225`, `8228` | Same arrival handoff for the non-chrono miner. | Yes |
| `ini/rulesmd.ini:[GAREFN]` | `DockUnload=yes`, `Refinery=yes` at lines `11726`, `11727` | Receiver case `0x15` queues mission `0x10`. | Yes |
| `ini/rulesmd.ini:[NAREFN]` | `DockUnload=yes`, `Refinery=yes` at lines `12519`, `12520` | Same as GAREFN. | Yes |
| `ini/rulesmd.ini` stock Bunker / UnitRepair flags | examples at lines `13732`, `11877`, `12665` | Other case branches; not stock refinery pad arrival. | Conditional / out-of-scope |

## 5. Current Rust Implementation Status

Rust models this as an explicit miner/refinery FSM rather than a raw radio/NavCom protocol. Relevant local surfaces are `src/sim/miner/mod.rs:86` (`RefineryDockPhase`) and `src/sim/miner/miner_dock_sequence.rs:538..559` (phase dispatch), with later transitions to `Linked`, `Pivoting`, and `Unloading` around `src/sim/miner/miner_dock_sequence.rs:596`, `654`, and `701`. No Rust files were modified.

## 6. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::PerCellProcess @ 0x00739EC0` hot dock-arrival branch | verified | decompile plus assembly context `0x0073A31F..0x0073A52B` | none for requested slice |
| Mission and destination gates | verified | `0x0073A31F..0x0073A359` | none |
| Cell equality timing | verified | `0x0073A36F..0x0073A437` | none |
| Locomotor/IPiggyback gate | verified | `0x0073A43D..0x0073A4CB` | exact naming of slot `+0x5C` outside this slot |
| `+0x5A4` read/write/compare | verified | `0x0073A4DF`, `0x0073A4E9`, `0x0073A4EF` | no stock-refinery dependency on conditional `+0x16A9` fill |
| Radio `0x15` send ordering | verified | `0x0073A4F7`, `0x0073A503`, `0x0073A507`, `0x0073A52B` | none |
| Building receiver case `0x15` | verified | `0x0043C2D0`, DockUnload branch; `0x005B35E0` mission queue callee | none for handoff edge |
| Radio `0x10` on standard arrival | verified absent | `0x00739EC0` hot branch; `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md` | broader TS-dead sender archaeology out-of-scope |
| `+0x418` fallback/cascade branch | verified as conditional | `0x0073A558..0x0073A5C8` | exact nonstandard trigger scenarios out-of-scope |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - What exact order fires on successful pad arrival? `FootClass::PerCellProcess(2)` -> radio `0x15` via vtable `+0x274` -> locomotor `+0x5C`. Evidence: `0x0073A4F7..0x0073A52B`.

[RESOLVED] OQ-2 - What are the gates before radio `0x15`? Current mission `7` or `0x19`, non-null building destination, dock-cell equality after `*0x100+0x80` normalization, non-null locomotor, and IPiggyback CLSID equal to `CLSID_WalkLocomotion`. Evidence: `0x0073A31F..0x0073A4CB`.

[RESOLVED] OQ-3 - Does this branch send radio `0x10`? No. The hot branch sends radio `0x15`; receiver-side mission queue uses mission id `0x10`. Evidence: `0x0073A503..0x0073A507`, `0x0043C2D0`, `0x005B35E0`.

[RESOLVED] OQ-4 - Does the fallback branch fire for successful stock CMIN refinery arrival? No for the taken `destination == unit+0x5A4` branch, because the function returns after locomotor `+0x5C`. The fallback branch is conditional and sends directed `0x15`, not `0x10`. Evidence: `0x0073A4EF..0x0073A52B`, `0x0073A558..0x0073A5C8`.

[DEFERRED] OQ-5 - Exact global semantic name of locomotor slot `+0x5C` across all locomotor implementations. Category: out-of-scope; this slot only needed ordering.

## Sources

- Ghidra `decompile_function 739ec0`; `get_assembly_context` for `0x0073A31F..0x0073A5C8`.
- Ghidra `decompile_function 0043c2d0`, `0065acb0`, `0065a970`, `006f4ab0`, `00737430`, `004d85d0`, `0073e5e0`, `00741970`, `005b35e0`.
- `ini/rulesmd.ini` local grep for `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`.
- Prior reports reconciled: `UNITCLASS_PERCELLPROCESS_PAD_ARRIVAL_FIELD_WRITES_GHIDRA_REPORT.md`, `miner/UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`, `miner/RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md`.
