# TypeClass+0xDFC LEAVE_DOCK Gate - Ghidra Research Report

**Address(es):** `UnitClass::Set_Destination` at `0x00741970`; `AircraftTypeClass::ReadINI` at `0x0041CC20`; `UnitTypeClass::ReadINI` at `0x00747620`; `TechnoClass::Receive_Radio` at `0x006F4AB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** identify the `TypeClass+0xDFC` field read in the `0x00741970` LEAVE_DOCK sender gate, determine its INI source, and decide whether it participates in standard YR CMIN/HARV refinery docking.  
**Non-Scope:** full carryall behavior, aircraft docking, transport pickup/drop-off choreography, and all other `Set_Destination` branches.  
**Confidence:** HIGH for the field identity and refinery applicability.  
**Active in YR:** Conditional. The code path exists in live `UnitClass::Set_Destination`, but this specific `+0xDFC` read is gated to an AircraftClass contact and is not active for standard CMIN/HARV-to-refinery contacts.

## 1. Overview

OQ-2 from `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` asked which INI key backs `TypeClass+0xDFC` in the `UnitClass::Set_Destination` LEAVE_DOCK gate. The answer is class-dependent: for `AircraftTypeClass`, `+0xDFC` is `Carryall=`, and the `Set_Destination` branch proves the contact is RTTI `2` before reading it. The same numeric offset in `UnitTypeClass` is `MovementRestrictedTo=`, but that is not the field consumed by this branch.

For normal refinery docking, CMIN/HARV contact a BuildingClass refinery (`WhatAmI()==6`) and use `DockUnload=yes` at `BuildingTypeClass+0x16B3`. They do not satisfy the AircraftClass contact gate, so `AircraftTypeClass+0xDFC Carryall` does not gate standard refinery LEAVE_DOCK or dock-link clearing.

## 2. Class Layout / Key Offsets

| Field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| object `+0x6C4` | type pointer read from the radio contact before `+0xDFC` | `0x00741B72` loads `[EAX+0x6C4]` after `FootClass__GetDestination` | Conditional: only after current contact exists and passes branch gates |
| `AircraftTypeClass+0xDFC` | `Carryall=` bool | `AircraftTypeClass::ReadINI @ 0x0041CC20` reads string `Carryall` at `0x00818028` and writes `param_1+0xDFC` | Yes as parsed aircraft type data; branch use is conditional |
| `UnitTypeClass+0xDFC` | `MovementRestrictedTo=` value | `UnitTypeClass::ReadINI @ 0x00747620` calls `FUN_004754B0(..., MovementRestrictedTo, old +0xDFC)` and writes `param_1+0xDFC` | Yes as unit type data, but not the field read in this branch |
| `BuildingTypeClass+0x16B3` | `DockUnload=` bool used by refinery CAN_DOCK success path | `UnitClass::Set_Destination @ 0x00741E10..0x00741E1E`; `rulesmd.ini:GAREFN/NAREFN DockUnload=yes` | Yes for standard GAREFN/NAREFN refinery docking |
| Techno docked flag around object `+0x2E4` | cleared by `TechnoClass::Receive_Radio` case `0x19` | `TechnoClass::Receive_Radio @ 0x006F4AB0`, case `0x19` clears the byte if set and propagates `0x19` | Conditional: only when a `0x19` message is actually received |

## 3. Core Logic

### 3.1 LEAVE_DOCK sender gate in `UnitClass::Set_Destination`

Verified branch order at `0x00741B30..0x00741BA4`:

1. `PathType__Has_Valid_Steps()` must be true.
2. vtable slot `+0x184` must return `0`.
3. `this+0x418` must be nonzero.
4. `FootClass__GetDestination(0)` returns the current radio contact.
5. The contact's `WhatAmI` vtable slot `+0x2C` must return `2`.
6. The contact's type pointer at `contact+0x6C4` is loaded.
7. Byte `type+0xDFC` must be nonzero.
8. The contact receives virtual `Set_Destination(NULL, 1)`.
9. This unit transmits `0x19` (LEAVE_DOCK) via radio slot `+0x274`.
10. This unit then transmits `0x03` (OVER_AND_OUT) via radio slot `+0x274`.

**Active in YR:** Conditional. The function is live for UnitClass objects, but the `+0xDFC` read requires a current contact with `WhatAmI()==2`. `AircraftClass` vtable slot `+0x2C` points to code at `0x0041C180` returning `2`; `BuildingClass::WhatAmI @ 0x00459EC0` returns `6`, and `UnitClass::What_Am_I @ 0x00746E20` returns `1`.

### 3.2 Why this is not a CMIN/HARV refinery gate

The refinery approach path in the same function filters the new destination, checks `WhatAmI()==6`, sends `CAN_DOCK(0x0E)` at `0x00741DDA`, and on ROGER reads `BuildingTypeClass+0x16B3 DockUnload` at `0x00741E10..0x00741E1E`. That is the standard harvester-refinery gate.

CMIN/HARV rules data point at buildings, not aircraft:

| INI section | Relevant keys | Evidence | Active in YR |
|---|---|---|---|
| `[CMIN]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` | `ini/rulesmd.ini:7361`, `ini/rulesmd.ini:7364` | Yes |
| `[HARV]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` | `ini/rulesmd.ini:8225`, `ini/rulesmd.ini:8228` | Yes |
| `[GAREFN]` | `DockUnload=yes`, `Refinery=yes`, `FreeUnit=CMIN` | `ini/rulesmd.ini:11726`, `11727`, `11736` | Yes |
| `[NAREFN]` | `DockUnload=yes`, `Refinery=yes`, `FreeUnit=HARV` | `ini/rulesmd.ini:12519`, `12520`, `12530` | Yes |

**Active in YR:** Yes for standard refinery docking, but through `BuildingTypeClass+0x16B3`, not `AircraftTypeClass+0xDFC`.

### 3.3 Receiver-side effect of LEAVE_DOCK

`TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x19` checks the docked byte, clears it if it was set, broadcasts/propagates `0x19`, and returns `1`. If the byte is already clear, it falls through to the base radio handler.

**Active in YR:** Conditional. This receiver behavior is live for TechnoClass-derived objects, but standard refinery release does not depend on this OQ-2 gate; prior refinery reports verified normal release clears dock fields directly and/or uses `0x03` paths, not this aircraft-carryall gate.

## 4. INI Keys

| INI key | Parsed class/offset | Binary evidence | Standard YR data | Active in YR |
|---|---|---|---|---|
| `Carryall=` | `AircraftTypeClass+0xDFC` bool | `0x0041CC20` reads `s_Carryall_00818028`; memory at `0x00818028` is `Carryall` | `[HIND] Carryall=yes` at `ini/rulesmd.ini:10822`; constructor default at `0x0041C8B0` is false | Yes as aircraft type data; conditional in this gate |
| `MovementRestrictedTo=` | `UnitTypeClass+0xDFC` value | `0x00747620` reads `s_MovementRestrictedTo_00845D64` into `+0xDFC` | not relevant to this branch | Yes as unit type data; no for this branch |
| `DockUnload=` | `BuildingTypeClass+0x16B3` bool | read at `0x00741E10..0x00741E1E` after `CAN_DOCK(0x0E)` ROGER | `GAREFN` and `NAREFN` set yes | Yes for standard refinery docking |

## 5. Integration Points

| Integration point | Finding | Active in YR |
|---|---|---|
| `UnitClass::Set_Destination @ 0x00741970` | Contains both the aircraft-carryall LEAVE_DOCK abort branch and the standard building CAN_DOCK branch | Yes; branch participation is conditional |
| `AircraftTypeClass::ReadINI @ 0x0041CC20` | Parses `Carryall=` into the exact byte read by the LEAVE_DOCK gate | Yes |
| `UnitTypeClass::ReadINI @ 0x00747620` | Proves same offset number is not globally one semantic; UnitType uses `MovementRestrictedTo=` at `+0xDFC` | Yes as parse data; not used here |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | Receives `0x19` and clears the docked byte before propagating | Conditional |
| Standard CMIN/HARV refinery docking | Uses building contact `WhatAmI()==6`, not aircraft contact `WhatAmI()==2` | Yes, and therefore bypasses the `Carryall` gate |

## 6. Current Rust Implementation Status

Not modified in this investigation. The narrow parity conclusion for implementers is that standard CMIN/HARV refinery docking should not add a `Carryall`/`TypeClass+0xDFC` gate. If a future carryall/aircraft-contact system is implemented, this branch belongs there, not in the normal refinery dock path.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Set_Destination` LEAVE_DOCK sender branch | verified | decompile and disassembly at `0x00741B30..0x00741BA4` | none for field identity |
| Contact RTTI requirement | verified | `CMP EAX,0x2` at `0x00741B64`; Aircraft vtable slot `+0x2C -> 0x0041C180` returns `2`; Building returns `6`; Unit returns `1` | none |
| `AircraftTypeClass+0xDFC` parser | verified | `AircraftTypeClass::ReadINI @ 0x0041CC20`; string `Carryall` at `0x00818028` | none |
| `UnitTypeClass+0xDFC` disambiguation | verified | `UnitTypeClass::ReadINI @ 0x00747620` parses `MovementRestrictedTo` into `+0xDFC` | none |
| Standard CMIN/HARV refinery applicability | verified | `Set_Destination` building branch at `0x00741DDA` / `0x00741E10`; `rulesmd.ini` CMIN/HARV/GAREFN/NAREFN lines | none |
| Full carryall transport behavior | deferred | out of scope | separate carryall investigation |

## 8. Open Questions - Final State

`[RESOLVED] OQ-DFC-001` - Which INI key corresponds to the `+0xDFC` read in the LEAVE_DOCK gate? Answer: `AircraftTypeClass Carryall=` because the read occurs only after contact `WhatAmI()==2`. Evidence: `0x00741B64`, `0x00741B72..0x00741B80`, `0x0041CC20`.

`[RESOLVED] OQ-DFC-002` - Is `UnitTypeClass+0xDFC` the same field? No. For UnitTypeClass, `+0xDFC` is `MovementRestrictedTo=`. Evidence: `UnitTypeClass::ReadINI @ 0x00747620`.

`[RESOLVED] OQ-DFC-003` - Does this gate standard CMIN/HARV refinery docking? No. Standard refinery docking uses a BuildingClass contact (`WhatAmI()==6`) and `BuildingTypeClass+0x16B3 DockUnload`. Evidence: `0x00741DDA`, `0x00741E10..0x00741E1E`, `rulesmd.ini:7361`, `8225`, `11726`, `12519`.

`[RESOLVED] OQ-DFC-004` - How does it affect LEAVE_DOCK/dock clearing? It gates the sender of `0x19` in this abort branch; when `0x19` is received, `TechnoClass::Receive_Radio` clears the docked byte if set. Evidence: `0x00741B82..0x00741BA4`, `0x006F4AB0` case `0x19`.

`[DEFERRED] OQ-DFC-005` - What player-visible carryall scenario reaches this exact branch? Category: out-of-scope. Next step: a dedicated carryall/aircraft-contact investigation.

## Sources

- Ghidra: `UnitClass::Set_Destination` / Ghidra label `TechnoClass__Set_Destination @ 0x00741970`
- Ghidra: `AircraftTypeClass::ReadINI @ 0x0041CC20`
- Ghidra: `AircraftTypeClass::Constructor @ 0x0041C8B0`
- Ghidra: `UnitTypeClass::ReadINI @ 0x00747620`
- Ghidra: `TechnoClass::Receive_Radio @ 0x006F4AB0`
- Ghidra: `BuildingClass::WhatAmI @ 0x00459EC0`, `UnitClass::What_Am_I @ 0x00746E20`, Aircraft vtable `+0x2C -> 0x0041C180`
- Repo INI: `ini/rulesmd.ini`
- Prior docs: `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`, `READINI_FIELD_MAPS.md`
