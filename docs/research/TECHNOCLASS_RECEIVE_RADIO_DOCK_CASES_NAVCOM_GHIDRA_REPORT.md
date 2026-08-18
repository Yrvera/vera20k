# TechnoClass::Receive_Radio Dock Cases / NavCom - Ghidra Research Report

**Address(es):** `0x006F4AB0` (primary), `0x0065A820`, `0x005F5320`, `0x0043C2D0`, `0x00737430`, `0x004D8FB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** TechnoClass receiver handling for dock/navcom message `0x18` ENTER_DOCK, `0x19` LEAVE_DOCK, and whether `0x10` has any TechnoClass handling or standard `CMIN -> GAREFN/NAREFN` meaning.  
**Non-Scope:** Full BuildingClass `0x0E` acceptance filters, full FootClass `0x12` movement fields, and per-cell arrival timing beyond citing sibling reports.  
**Confidence:** High  
**Active in YR:** Yes for `0x18`; Conditional for `0x19`; No meaningful TechnoClass handling for `0x10`.

## 1. Overview

`TechnoClass::Receive_Radio @ 0x006F4AB0` has direct handlers for dock-relevant `0x18` and `0x19`, but the field it toggles is `Techno/Foot/Unit +0x418`, not `+0x2E4`. Message `0x10` is not a TechnoClass case at all: the switch table maps it to the default path, which falls through to `RadioClass::Receive_Radio` and then `ObjectClass::Receive_Radio`, where `0x10` returns `0`.

For standard stock `CMIN/HARV -> GAREFN/NAREFN`, BuildingClass case `0x0E` sends `0x18` to the miner as part of the accepted dock reply burst. No live sender for radio `0x10` was found in the existing sender-trace report, and normal refinery exit does not use `0x19`; exit clears unrelated `+0x2E4` links directly only on the nonzero-link branch.

## 2. Class Layout / Key Offsets

| Field | Offset | Use in this slice | Active in YR |
|---|---:|---|---|
| `Techno/Foot/Unit +0x418` | `0x418` | Directly read/written by TechnoClass cases `0x18` and `0x19`; also read by sender-side dock logic as a destination/contact flag | Yes |
| `Techno/Unit/Building +0x2E4` | `0x2E4` | Not touched by TechnoClass `0x18/0x19`; stock refinery unload uses zero-`+0x2E4` path | Conditional, but not this handler |
| `AircraftTypeClass +0xE0D` | `0xE0D` | `AirportBound`; causes TechnoClass case `0x18` to skip its set/propagate block when receiver `WhatAmI()==2` | Yes for ORCA/BEAG; not for CMIN |
| TechnoClass vtable `+0x194` | `0x007F4AF4 -> 0x006F4AB0` | Receive_Radio override | Yes |
| UnitClass vtable `+0x194` | `0x007F5E04 -> 0x00737430` | UnitClass receives first, then falls through for `0x18/0x19` | Yes |
| FootClass vtable `+0x194` | `0x007E8E28 -> 0x004D8FB0` | FootClass falls through to TechnoClass for `0x18/0x19` | Yes |

## 3. Core Logic

### 3.1 Switch dispatch and case inventory

The TechnoClass switch normalizes `msg - 3`, bounds-checks `<= 0x1C`, indexes byte table `0x006F4E88`, then jumps through table `0x006F4E5C`.

Verified direct case set:

| Message | Table result | Handler |
|---:|---|---|
| `0x03` | direct | BREAK side-effect, then `RadioClass::Receive_Radio` |
| `0x07`, `0x09`, `0x16` | direct | transmit `0x18`, then `RadioClass::Receive_Radio`, return `1` |
| `0x08` | direct | transmit `0x19`, then `0x03`, return second transmit result |
| `0x18` | direct | set `+0x418=1` and propagate `0x18` if not already set |
| `0x19` | direct | clear `+0x418=0` and propagate `0x19` if set |
| `0x1A` / `0x1B` | direct | set/clear adjacent byte `+0x419`, propagate |
| `0x1C`, `0x1E`, `0x1F` | direct | repair/mission/counter handlers |
| `0x10` | default | no TechnoClass logic; call `RadioClass::Receive_Radio` |

Evidence: `read_memory 0x006F4E88` returns the 29-byte index table for messages `0x03..0x1F`; entry for message `0x10` is index `10`, whose jump target is default `0x006F4E3F`. `disassemble_function 0x006F4AB0` confirms default calls `0x0065A820`.

### 3.2 Case `0x18` ENTER_DOCK

Verified behavior:

1. If receiver `WhatAmI()==2`, load receiver type at `this+0x6C4` and check `Type+0xE0D` (`AirportBound` for AircraftTypeClass).
2. If that aircraft-only flag is set, skip the set/propagate block and fall through to default `RadioClass::Receive_Radio`.
3. Otherwise read `byte [this+0x418]`.
4. If `+0x418 == 0`, write `+0x418 = 1`, then call vtable `+0x278` with message `0x18` and target `sender`.
5. Return `1`.
6. If `+0x418 != 0`, do not write or propagate; fall through to `RadioClass::Receive_Radio`.

Load-bearing order: the byte write occurs before the propagation call. Evidence: assembly context around `0x006F4B72` shows `MOV byte ptr [ESI+0x418],0x1`, followed by `CALL [EDX+0x278]` at `0x006F4B79`.

**Active in YR:** Yes for standard CMIN refinery docking. BuildingClass case `0x0E @ 0x0043C2D0` sends `0x18` after `0x12` returns `0x14`, and CMIN is a UnitClass receiver, not an AircraftClass receiver. Stock CMIN and HARV are enabled harvesters with `Dock=NAREFN,GAREFN` (`rulesmd.ini:7361`, `rulesmd.ini:8225`); GAREFN/NAREFN have `DockUnload=yes` and `Refinery=yes` (`rulesmd.ini:11726..11727`, `12519..12520`).

### 3.3 Case `0x19` LEAVE_DOCK

Verified behavior:

1. Read `byte [this+0x418]`.
2. If `+0x418 != 0`, write `+0x418 = 0`, then call vtable `+0x278` with message `0x19` and target `sender`.
3. Return `1`.
4. If `+0x418 == 0`, do not write or propagate; fall through to `RadioClass::Receive_Radio`.

Load-bearing order: the clear occurs before the propagation call. Evidence: assembly context around `0x006F4BA6..0x006F4BAD` shows `MOV byte ptr [ESI+0x418],0x0`, then `CALL [EDX+0x278]`.

**Active in YR:** Conditional. The receiver logic is live for any Techno-derived object with `+0x418` set, but standard uninterrupted CMIN refinery unload does not use `0x19` for normal exit. Sibling reports identify `0x19` senders as TechnoClass BREAK side-effect and `UnitClass::Set_Destination @ 0x00741B97` cancel-dock, while `ReleaseDockedHarvester` and `UndockUnit` do not emit `0x19`.

### 3.4 Case `0x10` RESERVE_DOCK

Verified behavior:

1. TechnoClass has no direct `0x10` handler.
2. Message `0x10` enters default `0x006F4E3F`, which calls `RadioClass::Receive_Radio @ 0x0065A820`.
3. `RadioClass::Receive_Radio` handles only `0x03` and `0x02` directly, then delegates to `ObjectClass::Receive_Radio @ 0x005F5320`.
4. `ObjectClass::Receive_Radio` handles only `0x0D` and `0x22`; all other messages, including `0x10`, return `0`.

**Active in YR:** No as a TechnoClass behavior. BuildingClass does have a separate receiver case `0x10` at `0x0043C2D0` that can return ROGER for `Refinery=yes` buildings, but `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md` found no live radio `0x10` sender in the standard dock candidate set. The standard CMIN chain uses `0x0E` CAN_DOCK, not `0x10`.

## 4. Reconciliation With Prior Reports

| Prior source | Claim | Result of this slot |
|---|---|---|
| `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` | Sender-side `0x10` is absent from UnitClass/Set_Destination; normal CMIN admission uses `0x0E` | Corroborated. TechnoClass receiver also has no `0x10` case. |
| `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` OQ-10 | Whether TechnoClass handles `0x10` was open | Resolved: no TechnoClass `0x10`; it defaults to RadioClass/ObjectClass and returns `0`. |
| `RADIO_LINK...` message table / docked flag text | Says TechnoClass case `0x18/0x19` sets/clears `+0x2E4` | Corrected by this slot and by the later `STANDARD_REFINERY_0X2E4...` report: the direct TechnoClass field is `+0x418`, not `+0x2E4`. |
| `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md` | TechnoClass `0x18` writes `+0x418`, not `+0x2E4`; stock refinery path has no reciprocal `+0x2E4` writer | Corroborated with fresh disassembly at `0x006F4B72` and `0x006F4BA6`. |
| `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md` | Building receiver case `0x10` is live-in-principle but dead-send in standard YR | Corroborated for this slot; TechnoClass has no receiver-side `0x10` behavior either. |

## 5. Integration Points

- Standard accepted refinery `0x0E` path: `BuildingClass::Receive_Radio @ 0x0043C2D0` sends `0x12`, then `0x18`, then `0x16`. The `0x18` call reaches UnitClass -> FootClass -> TechnoClass, where `+0x418` is set and `0x18` is propagated once.
- UnitClass `0x16` then handles facing/timing and may cascade `0x15`; this slot did not re-investigate that case beyond confirming TechnoClass treats `0x16` as a `0x18` propagation trigger before base RadioClass handling.
- `0x19` is not the normal refinery exit signal. Normal stock unload exit is driven by `Mission_Deploy_Building`/release helpers and direct state writes, not by TechnoClass `0x19`.
- The TechnoClass propagation calls use vtable `+0x278`, which is `RadioClass::Transmit_Radio` per prior vtable binding docs.

## 6. Current Rust Implementation Status

Not inspected or modified in this slot. The relevant parity requirement from this binary slice is behavioral: accepted standard refinery dock should model the `+0x418`-like radio/contact flag transition on `0x18`, should not treat TechnoClass `0x18` as a `+0x2E4` reciprocal dock-link writer, and should not add a standard `0x10` reserve-dock step for CMIN refinery docking.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| TechnoClass switch table | verified | `read_memory 0x006F4E5C`, `0x006F4E88`; `disassemble_function 0x006F4AB0` | none |
| TechnoClass case `0x18` | verified | `0x006F4B1F..0x006F4B8A`; write at `0x006F4B72`, call at `0x006F4B79` | none |
| TechnoClass case `0x19` | verified | `0x006F4B8D..0x006F4BBE`; write at `0x006F4BA6`, call at `0x006F4BAD` | none |
| TechnoClass case `0x10` | verified absent | message `0x10` table entry -> default `0x006F4E3F`; default calls `0x0065A820` | none |
| RadioClass/ObjectClass fallback for `0x10` | verified | decompile `0x0065A820`, `0x005F5320` | none |
| BuildingClass case `0x10` distinction | touched-not-exhausted | decompile `0x0043C2D0`; prior sender trace | Full global 0x10 sender search remains sibling-doc territory |
| Standard CMIN refinery YR activity | verified via docs/INI | `rulesmd.ini:7361`, `8225`, `11726..11727`, `12519..12520`; BuildingClass `0x0E` decompile | none for this slice |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does `TechnoClass::Receive_Radio @ 0x006F4AB0` handle case `0x10`? No. The switch table maps `0x10` to default `0x006F4E3F`, which calls RadioClass/ObjectClass fallback; ObjectClass returns `0` for `0x10`. Evidence: `read_memory 0x006F4E88`, `disassemble_function 0x006F4AB0`, `decompile_function 0x0065A820`, `0x005F5320`.

[RESOLVED] OQ-2 - Do TechnoClass `0x18/0x19` write `+0x2E4`? No. They write byte `+0x418` only, before propagating the same message through vtable `+0x278`. Evidence: `0x006F4B72`, `0x006F4BA6`; corroborates `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`.

[RESOLVED] OQ-3 - Is case `0x18` active in the standard CMIN refinery chain? Yes. BuildingClass `0x0E` sends `0x18` in the stock DockUnload reply burst; CMIN/HARV dock with GAREFN/NAREFN by INI. Evidence: `0x0043C2D0`; `rulesmd.ini:7361`, `8225`, `11726..11727`, `12519..12520`.

[RESOLVED] OQ-4 - Is case `0x19` normal stock refinery exit? No. TechnoClass `0x19` is live but conditional; normal release/undock paths do not emit `0x19` and direct-clear other fields. Evidence: `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`; `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`.

[DEFERRED] OQ-5 - Exact semantic name for `+0x418` across all non-dock consumers. Category: out-of-scope. This slot verified the field writes/readers relevant to `0x18/0x19`; a broader struct-layout pass would be needed for all consumers.

## Sources

- Ghidra `decompile_function 0x006F4AB0`
- Ghidra `disassemble_function 0x006F4AB0`
- Ghidra `read_memory 0x006F4E5C`, `0x006F4E88`, `0x007F4AF4`, `0x007F5E04`, `0x007E8E28`
- Ghidra `get_assembly_context` for `0x006F4B72`, `0x006F4BAD`, `0x006F4E3F`, `0x006F4E4C`
- Ghidra `decompile_function 0x0065A820`, `0x005F5320`, `0x0043C2D0`, `0x00737430`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
