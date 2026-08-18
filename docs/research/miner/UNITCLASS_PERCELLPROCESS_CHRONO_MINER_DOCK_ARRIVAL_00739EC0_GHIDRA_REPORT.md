# UnitClass::PerCellProcess Chrono Miner Dock Arrival — Ghidra Research Report

**Address(es):** `0x00739EC0` primary (`UnitClass__PerCellProcess`), dock-arrival hot path at `0x0073A4D5..0x0073A52B`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The UnitClass per-cell dock-pad arrival handoff for standard YR harvesters, especially CMIN/HARV returning to GAREFN/NAREFN. This covers the immediate send/receive edge for radio `0x15`, the locomotion stop/update call, Mission 7 entry from harvest, and Mission `0x10` handoff to `UnitClass__Mission_Deploy_Building`.  
**Non-Scope:** Full radio protocol, full `Mission_Deploy_Building` dump loop, full chrono locomotor lifecycle, blocked-path behavior, production exit, and all aircraft/transport/garrison enter paths.  
**Confidence:** High for this slice.  
**Active in YR:** Yes — standard YR `[CMIN]` and `[HARV]` are `Harvester=yes` and `Dock=NAREFN,GAREFN`; standard `[GAREFN]` and `[NAREFN]` are `DockUnload=yes` and `Refinery=yes` in `ini/rulesmd.ini`.

## 1. Overview

The per-cell handler at `0x00739EC0` contains the dock-pad arrival pivot for a harvester already in the enter/dock approach path. When the unit's current cell equals the destination building's dock cell, the function updates the generic foot per-cell state, sends radio `0x15` to the current dock/radio contact, and immediately calls the locomotor vtable slot `+0x5C` to power off/stop the locomotor.

For standard DockUnload refineries, the receiver side is `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15`; it sets the sender unit to Mission `0x10`, whose UnitClass handler is `UnitClass__Mission_Deploy_Building @ 0x0073D630`. The value `0x10` is not sent as a radio message from this function.

## 2. Key Offsets And Slots

| Field / slot | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|
| Unit `+0x5A4` | DockLink / linked building for this dock handoff | `0x0073A4DF` reads and `0x0073A4E9` writes before comparing with destination | Yes |
| Unit `+0x674` | ILocomotion pointer | `0x0073A50D` null-check before locomotor call | Yes |
| UnitType `+0xE0E` | Harvester/chrono-style gatherer gate used by Mission_Harvest and later cleanup paths | `0x0073E5E0` Mission_Harvest gates on this; `[CMIN] Teleporter=yes`/harvester content reaches this path | Yes for CMIN, no for HARV on this chrono-specific flag |
| BuildingType `+0x16A9` | UnitRepair flag; only affects the optional DockLink fill in this branch | `0x0073A4D5` check before writing `+0x5A4` | Conditional; not GAREFN/NAREFN |
| BuildingType `+0x16B3` | `DockUnload=yes` | `BuildingClass__Receive_Radio` case `0x15` checks it; `rulesmd.ini:11726`, `12519` set it | Yes |
| BuildingType `+0x16BB` | `Refinery=yes` | `BuildingClass__Receive_Radio` case `0x10`; `rulesmd.ini:11727`, `12520` set it | Yes, but not used by this function's radio send |
| Unit vtable `+0x184` | current mission getter | `0x00739EC0` branch tests mission `7` or `0x19` before dock-cell compare | Yes |
| Unit vtable `+0x274` | radio send to existing contact / first linked destination | `0x0073A503 PUSH 0x15`; `0x0073A507 CALL [EDX+0x274]` | Yes |
| Unit/Techno vtable `+0x278` | directed radio send | Used elsewhere in same function for `0x0F`, `0x08`, `0x0E`; not for the dock-arrival `0x15` | Yes |
| ILocomotion vtable `+0x0C` | locomotor CLSID query | `0x0073A4AC..0x0073A4C7`, compared against `CLSID_WalkLocomotion` | Yes |
| ILocomotion vtable `+0x5C` | stop/power-off/process-stop call at arrival | `0x0073A521..0x0073A52B`; prior docs identify as `Power_Off` for this context | Yes |

## 3. Core Logic

### 3.1 Upstream Mission_Enter entry

`UnitClass__Mission_Harvest @ 0x0073E5E0` state 3 directly sets mission `7` with queued flag `0` (`0x0073EE8D PUSH 0`, `0x0073EE8F PUSH 0x7`, `0x0073EE93 CALL [EDX+0x1E8]`). This is the standard path that places a returning harvester into the enter/dock approach mission.

**Active in YR:** Yes. `[CMIN]` and `[HARV]` both set `Harvester=yes` and `Dock=NAREFN,GAREFN` in `rulesmd.ini`; this harvest FSM is the stock harvester route.

### 3.2 Dock-pad arrival predicate inside `0x00739EC0`

The hot path requires:

1. Current mission is `7` or `0x19`.
2. `FootClass__GetDestination()` returns a non-null target.
3. Destination `WhatAmI()` returns `6` (building).
4. The unit's current cell, converted from leptons to cell center using `*0x100 + 0x80`, equals the destination building's dock cell returned through the building vtable `+0xA8`.
5. The unit's locomotor exists and supports IPiggyback; the current piggyback CLSID is read and compared to `CLSID_WalkLocomotion`.

If the destination is a UnitRepair building (`BuildingType+0x16A9`) and Unit `+0x5A4` is empty, the function writes the destination building into `+0x5A4`. Standard GAREFN/NAREFN refineries do not depend on this optional repair-pad write because their DockLink has already been established by the admission radio path.

**Active in YR:** Yes for the predicate and equality gate. The `+0x16A9` write is Conditional and not active for standard ore refineries.

### 3.3 Exact arrival handoff sequence

When `destination == unit+0x5A4`, the sequence is:

1. Call `FootClass__PerCellProcess(2)` at `0x0073A4F7..0x0073A4FB`.
2. Send radio `0x15` through vtable `+0x274` at `0x0073A503..0x0073A507`.
3. Reload Unit `+0x674`; assert if null.
4. Call the locomotor vtable `+0x5C` at `0x0073A521..0x0073A52B`, documented in existing dock-arrival reports as `ILocomotion::Power_Off`.
5. Release the IPiggyback interface if present, then return.

The ordering matters: the generic foot per-cell side effects happen before radio `0x15`, and the locomotor stop/power-off happens after the radio send.

**Active in YR:** Yes. This is the standard pad-arrival event for stock CMIN/HARV docking with GAREFN/NAREFN.

### 3.4 Receiver-side Mission_Deploy_Building handoff

`BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15` checks the receiver building type. For `DockUnload=yes` (`+0x16B3`), it calls the sender unit's mission setter vtable `+0x1E8` with mission `0x10` and queued flag `0`, then returns `1`.

`UnitClass__Mission_Deploy_Building @ 0x0073D630` is the UnitClass mission handler for Mission `0x10`. Its harvester branch owns the unload FSM: initialization, per-slot cargo drain, dock animation updates, and later exit handoff. This report only verifies the entry edge from pad arrival.

**Active in YR:** Yes. GAREFN/NAREFN have `DockUnload=yes`; CMIN/HARV target them through `Dock=NAREFN,GAREFN`.

### 3.5 Does `0x00739EC0` send radio `0x10`?

No. In this function, verified radio sends include:

| Radio code | Context | Evidence | Active in YR |
|---:|---|---|---|
| `0x15` | Dock-pad arrival, after `FootClass__PerCellProcess(2)` | `0x0073A503 PUSH 0x15`; call vtable `+0x274` | Yes |
| `0x0F` | Generic enter / passenger-style branches | decompile `0x00739EC0` uses vtable `+0x278` | Conditional; not the standard refinery handoff |
| `0x08` | Queue/admission check branch | decompile `0x00739EC0` uses vtable `+0x274` | Yes, but earlier than pad arrival |
| `0x0E` | Re-contact existing dock target in a branch | decompile `0x00739EC0` uses vtable `+0x278` | Conditional |
| `0x03` | Cleanup/cancel in late chrono/refinery checks | decompile `0x00739EC0` uses vtable `+0x274` | Conditional |

No `PUSH 0x10` is paired with vtable `+0x274`, `+0x278`, or `+0x27C` in this function. The nearby and easy-to-confuse `0x10` is a mission id set by `BuildingClass__Receive_Radio` after it receives radio `0x15`.

**Active in YR:** No for radio `0x10` from this function. Mission `0x10` is active and standard.

## 4. INI Keys

| INI path | Values | Effect for this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN]` | `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=20`, `UnloadingClass=CMON`, `Teleporter=yes`, teleport locomotor | CMIN uses the harvester dock path; its chrono behavior affects how it reaches the pad, not the final pad-arrival handoff | Yes (`rulesmd.ini:7351`, `7361`, `7364`, `7374`, `7384`, `7396`, `7398`) |
| `rulesmd.ini:[HARV]` | `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=40`, `UnloadingClass=HORV`, drive locomotor | HARV uses the same pad-arrival handoff without chrono-specific locomotor | Yes (`rulesmd.ini:8215`, `8225`, `8228`, `8236`, `8246`, `8258`) |
| `rulesmd.ini:[GAREFN]` | `DockUnload=yes`, `Refinery=yes`, `Storage=200` | Building receiver case `0x15` uses DockUnload to set sender mission `0x10` | Yes (`rulesmd.ini:11722`, `11726`, `11727`, `11744`) |
| `rulesmd.ini:[NAREFN]` | `DockUnload=yes`, `Refinery=yes`, `Storage=200` | Same as GAREFN | Yes (`rulesmd.ini:12515`, `12519`, `12520`, `12538`) |
| `rulesmd.ini:[YAREFN]` | Slave miner building, no `DockUnload=yes` in the checked lines | Not part of standard HARV/CMIN DockUnload handoff | No for this slice |

## 5. Integration Points

| Function | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass__Mission_Harvest @ 0x0073E5E0` | Sets Mission `7` after return-to-refinery state reaches EnterDock | `0x0073EE8D..0x0073EE93` | Yes |
| `UnitClass__PerCellProcess @ 0x00739EC0` | Detects dock-cell arrival and sends radio `0x15` | `0x0073A4D5..0x0073A52B` | Yes |
| `FootClass__PerCellProcess @ 0x004D85D0` | Generic per-cell state update called with arg `2` before radio send | call at `0x0073A4FB` | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Receiver case `0x15` sets sender mission `0x10` when `DockUnload=yes` | decompile case `0x15`; docs confirm vtable `+0x1E8` call | Yes |
| `UnitClass__Mission_Deploy_Building @ 0x0073D630` | Mission `0x10` handler for harvester unload | decompile and `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` | Yes |
| `UnitClass__Receive_Radio @ 0x00737430` case `0x16` | Separate timing cascade can send `0x15` when locomotor is not moving and mission is `7`; not the same pad-arrival send | `0x00737776..0x0073777A` | Yes, but separate edge |
| `TechnoClass__Set_Destination @ 0x00741970` | Earlier approach/admission path; sends/cancels other radio codes, not this final `0x15` | decompile shows `0x19` cancel and `0x0E` docking contacts | Yes |

## 6. Current Rust Implementation Status

The Rust sim has a separate refinery dock FSM rather than the raw radio protocol. Relevant surfaces:

| Area | Rust evidence | Status vs this binary slice |
|---|---|---|
| `RefineryDockPhase` | `src/sim/miner/mod.rs:84` | Represents Approach/Linked/Pivoting/Unloading/DepositCooldown/Departing instead of Mission 7 + radio `0x15` + Mission `0x10` |
| Pad-arrival transition | `src/sim/miner/miner_dock_sequence.rs:530..561` | Explicitly treats pivot completion as the radio `0x15` equivalent and seeds unload timing |
| UnloadingClass visual | `src/sim/miner/miner_dock_sequence.rs:512..525` | Applies `UnloadingClass` on linked/dock deploy |
| Deposit loop | `src/sim/miner/miner_dock_sequence.rs:575..669` | Implements the Mission_Deploy_Building analogue |
| World tick order | `src/sim/world/mod.rs:1429..1448` | Docks tick in the late sim phase |

No Rust files were modified in this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass__PerCellProcess @ 0x00739EC0` dock-arrival branch | verified | decompile + assembly context `0x0073A4D5..0x0073A52B` | none for requested slice |
| Radio `0x15` send ordering | verified | `0x0073A4F7` then `0x0073A503` then `0x0073A52B` | none |
| Radio `0x10` from this function | verified absent | decompile `0x00739EC0`; sender-trace doc says this function has no `0x10` radio send | none |
| `BuildingClass__Receive_Radio` case `0x15` DockUnload handoff | verified | decompile `0x0043C2D0`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md:316..340` | none |
| Mission `0x10` handler identity | verified | `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md:18`, decompile `0x0073D630` | full dump loop out-of-scope |
| `UnitClass__Receive_Radio` case `0x16` cascade | touched-not-exhausted | decompile `0x00737430`; assembly `0x00737776..0x0073777A` | full protocol out-of-scope |
| `TechnoClass__Set_Destination @ 0x00741970` | touched-not-exhausted | decompile read for immediate approach edge | broader destination semantics out-of-scope |
| YR stock content activation | verified | `rulesmd.ini` line evidence for CMIN/HARV/GAREFN/NAREFN | none |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 — What exact sequence fires at pad arrival? `FootClass__PerCellProcess(2)` at `0x0073A4F7..0x0073A4FB`, then radio `0x15` at `0x0073A503..0x0073A507`, then locomotor vtable `+0x5C` at `0x0073A521..0x0073A52B`.

[RESOLVED] OQ-2 — Does `0x00739EC0` send radio `0x10`? No. It sends `0x15` on this path; `0x10` is a mission id set by the receiver after radio `0x15`.

[RESOLVED] OQ-3 — Does the handoff reach `Mission_Deploy_Building`? Yes. `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15` calls sender mission setter with `0x10`; `UnitClass__Mission_Deploy_Building @ 0x0073D630` is the mission `0x10` handler.

[RESOLVED] OQ-4 — Is the path active for standard YR CMIN/HARV? Yes. `[CMIN]` and `[HARV]` both use `Harvester=yes` plus `Dock=NAREFN,GAREFN`; `[GAREFN]` and `[NAREFN]` both use `DockUnload=yes` plus `Refinery=yes`.

[RESOLVED] OQ-5 — Is the optional `BuildingType+0x16A9` DockLink write part of standard refinery DockUnload? No. It is conditional UnitRepair handling; standard refineries use the established DockLink/admission chain and the DockUnload receiver case.

[DEFERRED] OQ-6 — Exact semantic name of ILocomotion vtable `+0x5C` across all locomotor implementations. Existing dock docs call it `Power_Off`; this report only needs its ordering and call site. Category: out-of-scope.

## Sources

- Ghidra `decompile_function 0x00739EC0` — primary per-cell handler.
- Ghidra `get_assembly_context` around `0x0073A4F7`, `0x0073A503`, `0x0073A52B`.
- Ghidra `decompile_function 0x0043C2D0` — `BuildingClass__Receive_Radio`.
- Ghidra `decompile_function 0x00737430` — `UnitClass__Receive_Radio`.
- Ghidra `decompile_function 0x0073D630` — `UnitClass__Mission_Deploy_Building`.
- Ghidra `decompile_function 0x004D85D0` — `FootClass__PerCellProcess`.
- Ghidra `decompile_function 0x0073E5E0` — `UnitClass__Mission_Harvest`.
- Ghidra `decompile_function 0x00741970` — `TechnoClass__Set_Destination`.
- `ini/rulesmd.ini` lines for CMIN/HARV/GAREFN/NAREFN activation.
- Prior docs: `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`.
