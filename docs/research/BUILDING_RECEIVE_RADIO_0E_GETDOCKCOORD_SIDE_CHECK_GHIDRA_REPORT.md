# Building Receive Radio 0x0E GetDockCoord Side-Check - Ghidra Report

**Address(es):** `0x0043C2D0` primary, focused range `0x0043C8E2..0x0043C93A`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the early `GetDockCoord` touch inside `BuildingClass::Receive_Radio(0x0E)` before the hardcoded `NW+(3,1)` `0x12` payload, including the requester `+0x5A4` compare and stack sentinel effect.
**Non-Scope:** full tick-order winner between `0x16`-sourced `0x15` and `PerCellProcess`-sourced `0x15`, full locomotor movement internals, and Rust implementation patches.
**Confidence:** High for the side-check mechanism and its direct control-flow effect; Medium for exact runtime frequency because tick-order is assigned to sibling swarm slots.
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN` after contact is present/created, because stock refineries set `DockUnload=yes` (`BuildingType+0x16B3`) and stock harvesters target them.

## 0. Working Notes / Contract

**Target question:** Decode `BuildingClass::Receive_Radio(0x0E)` early `GetDockCoord` side-check at `0x0043C8E2..0x0043C93A`: requester fields, `+0x5A4` compare, stack sentinel, and whether it changes the subsequent radio flow.
**Non-goals:** Do not re-prove accepted `NW+(3,1)` except as contrast; do not settle global tick order; do not patch Rust or in-repo docs.
**Evidence needed to mark COMPLETE:** decompile plus assembly for `0x0043C8E2..0x0043C93A`, sentinel consumer at `0x0043CA07..0x0043CA0D`, `FootClass::Receive_Radio(0x13)` requester response, `GetDockCoord`, INI liveness, and Rust surface scan.
**Stop conditions:** Stop once the side-check's inputs, output byte, direct branch effect, negative effects, and handoff risk are all documented with no open items.

## 1. Overview

The early `GetDockCoord` touch is not the accepted movement target. It computes the building's dock coordinate, converts it to a `CellClass*`, compares that against the requester's `FootClass+0x5A4` NavCom/destination pointer, and sets a local stack byte only when the requester already has a different non-null destination.

That local byte is consumed later, after the building asks the requester `NEED_TO_MOVE(0x13)`. If the requester answers non-ROGER, the byte decides whether the building returns early or continues into the same accepted `0x12 -> 0x18 -> 0x16` flow. It does not replace the later hardcoded `NW+(3,1)` `0x12` payload and does not write persistent object state.

## 2. Key Offsets / Fields

| Owner | Offset / slot | Meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x16B3` | `DockUnload=yes`; enables this side-check for stock GAREFN/NAREFN | Yes | `0x0043C8E8..0x0043C8F0`; `rulesmd.ini:11726`, `12519` |
| `BuildingTypeClass` | `+0x16BC` | `Weeder=yes`; alternative predicate sharing the side-check | No for stock GAREFN/NAREFN | `0x0043C8F2..0x0043C8FA`; stock INI lacks `Weeder=` |
| requester object | `+0x14 & 0x04` | requester must be Foot-derived before being passed to `GetDockCoord` | Yes for CMIN/HARV | `0x0043C904..0x0043C90F` masks `EDI` to `EBP` |
| requester Foot | `+0x5A4` | NavCom/destination pointer compared to the computed dock `CellClass*` | Yes | `0x0043C92C`; `FootClass::Set_Destination_Internal @ 0x004D94B0` writes `param_1[0x169]` |
| local stack byte | `[ESP+0x54]` / initialized at `[ESP+0x58]` before pushes | allowance sentinel for a non-ROGER `0x13` response | Yes | init `0x0043C8D4`, write `0x0043C93A`, read `0x0043CA07` |
| building vtable | `+0xA8` | `BuildingClass::GetDockCoord` call | Yes | call `0x0043C91B`; body `0x00447B20` |
| map singleton | `0x0087F7E8` | `MapClass::Get_CellClass` receiver | Yes | `0x0043C922..0x0043C927` |

## 3. Core Logic

### 3.1 Side-check preconditions

Before the focused range, the receiver has already run contact checks. At `0x0043C8D4`, the local allowance byte is initialized to `0`. At `0x0043C8D9`, `DynamicVectorClass::Contains(this, requester)` must return true or the side-check is skipped.

At `0x0043C8E2..0x0043C8FA`, the side-check then requires `building.Type+0x16B3 DockUnload` or `building.Type+0x16BC Weeder`. This is active for stock GAREFN/NAREFN through `DockUnload=yes`, not through `Weeder`.

**Active in YR:** Yes, conditional on the requester being in the building contact list. The normal admitted path can create/check that contact earlier in the same `0x0E` case. Evidence: `0x0043C8C3` HELLO send, `0x0043C8CC/0x0043C8D9` contact contains checks, `FUN_0065ADF0`, `rulesmd.ini:11726`, `12519`.

### 3.2 Requester pointer narrowing

The code narrows the requester before calling `GetDockCoord`:

```text
if requester == null:
    foot_arg = 0
else:
    foot_arg = (requester+0x14 & 0x04) ? requester : 0
building.GetDockCoord(out_coord, foot_arg)
```

Assembly proof:

- `0x0043C8FC..0x0043C902`: null requester becomes `EBP=0`.
- `0x0043C904..0x0043C90F`: reads `[EDI+0x14]`, masks bit `0x04`, neg/sbb expands it to all-ones/all-zeroes, then `AND EAX, EDI` into `EBP`.
- `0x0043C917..0x0043C91B`: pushes `EBP` and calls building vtable `+0xA8`.

**Active in YR:** Yes for CMIN/HARV because they are Foot-derived requesters. Evidence: assembly range above plus stock `CMIN/HARV Dock=NAREFN,GAREFN` at `rulesmd.ini:7361`, `8225`.

### 3.3 Compare and sentinel write

The call sequence is:

```text
dock_coord = building.GetDockCoord(foot_arg)
dock_cell = MapClass.Get_CellClass(dock_coord)
navcom = foot_arg->+0x5A4
if navcom != 0 and navcom != dock_cell:
    local_allow_non_roger_13 = 1
```

Assembly proof:

- `0x0043C91B`: `CALL dword ptr [EDX + 0xA8]` (`GetDockCoord`).
- `0x0043C921..0x0043C927`: pushes the returned coordinate and calls `CellClass__Get_Cell_At @ 0x00565730`.
- `0x0043C92C`: loads `EBP = [EBP + 0x5A4]`.
- `0x0043C932..0x0043C938`: skips if `+0x5A4` is null or equals the computed dock `CellClass*`.
- `0x0043C93A`: writes `1` to the local stack byte.

**Active in YR:** Yes for stock refinery admission attempts with a Foot requester. The write is conditional: it fires only when the requester's current NavCom is non-null and not exactly the `CellClass*` for `GetDockCoord`.

### 3.4 The only verified consumer

The byte is read after the building sends `NEED_TO_MOVE(0x13)`:

```text
reply = building.Transmit(0x13, requester)
if reply != 1 and local_allow_non_roger_13 == 0:
    return 1
continue into accepted flow
```

Assembly proof:

- `0x0043C9F5..0x0043C9FC`: sends directed `0x13`.
- `0x0043CA02..0x0043CA05`: if reply is `1`, continue regardless of sentinel.
- `0x0043CA07..0x0043CA0D`: if reply is not `1`, read local byte; if zero, jump to early return path `0x0043CCF2`.
- `0x0043CA13`: continued path writes `*param_4 = this`, then later computes the accepted payload.

`FootClass::Receive_Radio(0x13) @ 0x004D8FB0` writes `*param_4 = Foot+0x5A4`; if `+0x5A4` is non-null and the locomotor reports moving, it returns `10`, otherwise it returns `1`.

**Active in YR:** Yes. This is a live synchronous radio exchange on the stock receiver path. Evidence: `0x0043C9F5..0x0043CA0D`; `FootClass::Receive_Radio @ 0x004D8FB0` case `0x13`; `RadioClass__Transmit_Radio_Impl @ 0x0065A970` forwards non-HELLO/non-BREAK messages synchronously.

### 3.5 What happens after continuation

If the code continues, the accepted stock `DockUnload || Weeder` block still computes:

```text
packed = building.Get_Cell_Packed()
payload_cell = packed + (3, 1)
payload = MapClass.Get_CellClass(payload_cell)
send 0x12
if reply == 0x14:
    send 0x18
    send 0x16
```

Assembly proof:

- `0x0043CA71..0x0043CA8D`: packed building cell plus `+3,+1`.
- `0x0043CAA3..0x0043CAAE`: `MapClass::Get_CellClass`, write payload pointer.
- `0x0043CAB2..0x0043CAB8`: send `0x12`.
- `0x0043CABE..0x0043CAC1`: require `0x14`.
- `0x0043CACA..0x0043CADB`: send `0x18`, then `0x16`.

**Active in YR:** Yes for stock `DockUnload=yes` refineries. The side-check only gates whether this later sequence can proceed after a non-ROGER `0x13`; it does not alter the payload cell.

## 4. INI Keys

| File | Section | Key | Stock value | Effect here | Active in YR |
|---|---|---|---|---|---|
| `rulesmd.ini` | `[CMIN]` | `Dock` | `NAREFN,GAREFN` | makes CMIN a stock requester | Yes, `7361` |
| `rulesmd.ini` | `[CMIN]` | `Harvester` | `yes` | stock harvester behavior | Yes, `7364` |
| `rulesmd.ini` | `[HARV]` | `Dock` | `NAREFN,GAREFN` | makes HARV a stock requester | Yes, `8225` |
| `rulesmd.ini` | `[HARV]` | `Harvester` | `yes` | stock harvester behavior | Yes, `8228` |
| `rulesmd.ini` | `[GAREFN]` | `DockUnload` | `yes` | enables side-check and accepted `DockUnload` branch | Yes, `11726` |
| `rulesmd.ini` | `[NAREFN]` | `DockUnload` | `yes` | enables side-check and accepted `DockUnload` branch | Yes, `12519` |
| `rulesmd.ini` | `[GAREFN]/[NAREFN]` | `Refinery` | `yes` | not the predicate for this side-check | Yes elsewhere, `11727`, `12520` |
| `artmd.ini` | `[GAREFN]/[NAREFN]` | `QueueingCell` | `4,1` | not read by this side-check | Conditional elsewhere, `1716`, `1773` |

## 5. Integration Points

| Function | Role | Active in YR | Evidence |
|---|---|---|---|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | owns the side-check and later accepted payload | Yes | decompile plus assembly contexts above |
| `BuildingClass::GetDockCoord @ 0x00447B20` | computes side-check coordinate | Yes for this caller when preconditions pass | vtable `+0xA8` call `0x0043C91B`; decompile `0x00447B20` |
| `CellClass::Get_Cell_At @ 0x00565730` | converts dock coordinate to `CellClass*` for compare | Yes | call `0x0043C927`; decompile `0x00565730` |
| `FootClass::Receive_Radio @ 0x004D8FB0` | `0x13` response reads `Foot+0x5A4` and moving state | Yes | case `0x13` decompile |
| `FootClass::Set_Destination_Internal @ 0x004D94B0` | proves `+0x5A4` is NavCom/destination | Yes | writes `param_1[0x169] = param_2` |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | synchronous directed radio dispatch | Yes | decompile |

## 6. Current Rust Implementation Status

No Rust files were modified. Current Rust surfaces found through Codegraph and source reads:

| Rust surface | Status against this slice |
|---|---|
| `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` | Missing/unchecked for the exact early `GetDockCoord` side-check and non-ROGER `0x13` allowance byte. Current flow has direct phase transitions around accepted cell movement/linking. |
| `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell` | Correctly keeps accepted `0x12` payload as `NW+(3,1)`; this report does not change that. |
| `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell` | Represents a separate `GetDockCoord`/pad concept; must not be substituted for accepted `0x12`. |
| `src/sim/miner/miner_system.rs::refinery_dock_cell` | Delegates to accepted `NW+(3,1)` helper; naming remains risky because it can be confused with `GetDockCoord`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| side-check preconditions | verified | `0x0043C8D4..0x0043C8FA` | none |
| requester foot-mask | verified | `0x0043C8FC..0x0043C90F` | exact semantic name of bit `0x04` not re-audited beyond Foot-derived requester use |
| `GetDockCoord` call and map conversion | verified | `0x0043C911..0x0043C927`; `0x00447B20`; `0x00565730` | none |
| requester `+0x5A4` compare | verified | `0x0043C92C..0x0043C93A`; `0x004D94B0` | none |
| sentinel consumer | verified | `0x0043C9F5..0x0043CA0D`; `0x004D8FB0` | sibling slots cover tick-order consequences |
| later accepted `0x12/0x18/0x16` flow unchanged | verified | `0x0043CA71..0x0043CADB` | none for this branch |
| stock YR liveness | verified | `rulesmd.ini:7361`, `7364`, `8225`, `8228`, `11726`, `12519` | none |
| Rust comparison | touched-not-exhausted | Codegraph + source reads | implementation tests deferred to parent |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the side-check active for stock YR refineries? -> Yes, through `DockUnload=yes` on GAREFN/NAREFN, when the requester is in/added to contacts.` (evidence: `0x0043C8D9..0x0043C8F0`; `rulesmd.ini:11726`, `12519`)
- `[RESOLVED] OQ-02 - Which requester field is involved? -> Foot requester `+0x5A4` NavCom/destination, after requester is masked by `+0x14 & 0x04`.` (evidence: `0x0043C904..0x0043C92C`; `0x004D94B0`)
- `[RESOLVED] OQ-03 - What is compared to `+0x5A4`? -> `CellClass*` for the building `GetDockCoord` coordinate, not raw coords.` (evidence: `0x0043C91B..0x0043C927`; `0x00565730`)
- `[RESOLVED] OQ-04 - What does the sentinel mean? -> Allow continuation after a non-ROGER `0x13` when requester `+0x5A4` is non-null and differs from `GetDockCoord` cell.` (evidence: `0x0043C93A`; `0x0043CA07..0x0043CA0D`)
- `[RESOLVED] OQ-05 - Does the side-check change the accepted `0x12` payload? -> No, later payload remains hardcoded NW+(3,1).` (evidence: `0x0043CA71..0x0043CAB8`)
- `[RESOLVED] OQ-06 - Does the side-check send `0x18` or `0x16` itself? -> No, it only sets a local byte consumed before the later send sequence.` (evidence: `0x0043C8E2..0x0043C93A`; `0x0043CACA..0x0043CADB`)
- `[RESOLVED] OQ-07 - Does it write requester/building persistent fields? -> No persistent write observed in the slice; only local stack byte is written.` (evidence: assembly range `0x0043C8E2..0x0043C93A`)
- `[RESOLVED] OQ-08 - Does it use `QueueingCell` or `DockingOffset`? -> No.` (evidence: no reads in focused range; contrast `0x00447B20` for dock offsets)
- `[RESOLVED] OQ-09 - How does `0x13` interact? -> Foot `0x13` returns `10` when `+0x5A4` is non-null and locomotor is moving; side-check sentinel can override the receiver's early return after that non-ROGER reply.` (evidence: `0x004D8FB0`; `0x0043CA02..0x0043CA0D`)
- `[DEFERRED] OQ-10 - Which `0x15` source wins first in stock tick order?` (category: `out-of-scope`; reason: assigned to sibling swarm slots; next-step-if-pursued: reconcile slot 1/2/3/5 reports)
- `[DEFERRED] OQ-11 - Exact runtime frequency of `+0x5A4 == GetDockCoord CellClass*` equality for stock miners.` (category: `needs-runtime-debugger`; reason: requires observing live NavCom object pointer values across repeated `0x0E` attempts; next-step-if-pursued: watch requester `+0x5A4` and branch `0x0043C936`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Receiver `0x0E` computes a `GetDockCoord` `CellClass*` side-check and sets a local allowance only if requester `NavCom(+0x5A4)` is non-null and different. | `0x0043C91B..0x0043C93A`; `0x004D94B0` | missing/unchecked | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` | Track the `0x13` precheck distinction instead of treating accepted contact as an unconditional link. | `refinery_candock_side_check_allows_busy_requester_with_different_navcom` | Do not implement by changing accepted target to `NW+(2,1)`. |
| If `0x13` reply is non-ROGER and the side-check byte is zero, building returns `1` before `0x12/0x18/0x16`; if byte is one, it continues. | `0x0043C9FC..0x0043CA0D` | likely missing | same | Model the early-return gate around `NEED_TO_MOVE`, especially while miner is moving to an existing destination. | `refinery_candock_non_roger_13_without_sidecheck_skips_enter_burst` | Do not always emit the 0x18/0x16 equivalent after HELLO/contact admission. |
| The later accepted payload remains `NW+(3,1)` after the side-check. | `0x0043CA71..0x0043CAB8` | none for accepted helper | `refinery_can_dock_queue_cell`, `refinery_dock_cell` naming | Preserve accepted-cell helper; consider naming split so side-check/GetDockCoord and accepted payload cannot be conflated. | `refinery_sidecheck_does_not_change_move_to_cell_payload` | Do not fold `GetDockCoord`, accepted cell, and `QueueingCell` into one helper. |

## 10. Negative Facts / Do Not Do

- Do not treat the early `GetDockCoord` result as the `0x12` movement payload. Evidence: payload is rebuilt later at `0x0043CA71..0x0043CAB8`.
- Do not ignore the side-check as dead: it controls the early return after a non-ROGER `0x13`. Evidence: `0x0043CA07..0x0043CA0D`.
- Do not make the side-check persistent state. Evidence: the focused range writes only a stack byte at `0x0043C93A`.
- Do not say the side-check is Weeder-only. Evidence: predicate is `DockUnload || Weeder`; stock refineries set `DockUnload=yes`.
- Do not use `QueueingCell=4,1` or `DockingOffset0` for this side-check. Evidence: no reads in `0x0043C8E2..0x0043C93A`.

## 11. Remaining Uncertainty

- Which `0x15` source wins first in stock tick order remains outside this slot.
- The exact live equality frequency for `requester+0x5A4 == GetDockCoord CellClass*` needs runtime watchpoints if parent wants absolute replay timing proof.

## 12. Stale Docs / Follow-up Wording

- Replace any claim saying "the early `GetDockCoord` touch changes the accepted refinery target to NW+(2,1)" with: "The early `GetDockCoord` touch in `BuildingClass::Receive_Radio(0x0E)` is a side-check against the requester's `+0x5A4` NavCom and a local allowance byte for the later `0x13` response. The accepted `0x12` payload remains hardcoded building NW+(3,1)."
- Replace any claim saying "the side-check has no effect" with: "The side-check has no effect on payload coordinates, but it can allow the `0x12/0x18/0x16` burst to proceed after a non-ROGER `0x13` response when the requester's current NavCom differs from the `GetDockCoord` cell."

## Sources

- Ghidra decompile: `BuildingClass__Receive_Radio @ 0x0043C2D0`.
- Ghidra assembly context: `0x0043C8D4`, `0x0043C8E2..0x0043C93A`, `0x0043C9F5..0x0043CA0D`, `0x0043CA71..0x0043CADB`.
- Ghidra decompile: `BuildingClass__GetDockCoord @ 0x00447B20`.
- Ghidra decompile: `CellClass__Get_Cell_At @ 0x00565730`.
- Ghidra decompile: `FootClass__Receive_Radio @ 0x004D8FB0`.
- Ghidra decompile: `FootClass__Set_Destination_Internal @ 0x004D94B0`.
- Ghidra decompile: `RadioClass__Transmit_Radio_Impl @ 0x0065A970`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`.
- Codegraph context and source reads for `src/sim/miner/miner_dock_sequence.rs` and `src/sim/miner/miner_system.rs`.

