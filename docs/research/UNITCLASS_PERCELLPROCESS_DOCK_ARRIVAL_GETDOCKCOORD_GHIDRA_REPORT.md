# UnitClass::PerCellProcess Dock Arrival GetDockCoord - Ghidra Research Report

**Address(es):** `0x00739EC0` primary, `0x00447B20` supporting `BuildingClass__GetDockCoord`, `0x0043C2D0` supporting building radio receiver  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Verify whether standard YR `CMIN/HARV -> GAREFN/NAREFN` dock arrival in `UnitClass__PerCellProcess` calls destination building vtable `+0xA8`, compares current cell to that dock coordinate, and how that relates to accepted `0x0E` `NW+(3,1)` versus disputed `NW+(2,1)`.  
**Non-Scope:** Full `BuildingClass__Receive_Radio` switch, full `UnitClass__Receive_Radio(0x16)` timing-sync path, full unload FSM, full BuildingType parser flag audit, and non-refinery docks.  
**Confidence:** High for the PerCellProcess arrival gate and stock `GetDockCoord` cell result; Medium for the exact upstream movement/choreography between `0x0E` accepted cell and the pad-arrival gate.  
**Active in YR:** Yes. Standard `[CMIN]`/`[HARV]` use `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]`/`[NAREFN]` use `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `Foundation=4x3`, and active refinery art.

> **Correction 2026-05-24 - relation to `0x16`**
>
> The PerCellProcess `GetDockCoord` equality branch documented here is real and
> active, but the implementation handoff below is too strong where it implies
> every unload handoff must wait for physical/current `NW+(2,1)`. A later or
> already-synced `UnitClass::Receive_Radio(0x16)` can send `0x15` from a stopped
> accepted-cell state without calling `GetDockCoord`, setting a destination, or
> writing location. Correct current handoff: gate unload on a verified `0x15`
> source. Possible verified sources include later/already-synced `0x16` from
> accepted `NW+(3,1)`, this PerCellProcess `GetDockCoord` equality branch at
> `NW+(2,1)`, and the later `+0x418` adjacent-building PerCellProcess branch.

## 0. Working Notes

- Target question: Does `UnitClass__PerCellProcess @ 0x00739EC0` use destination BuildingClass vtable `+0xA8`/`GetDockCoord` for stock miner refinery dock arrival, and how does that relate to `NW+(3,1)` vs `NW+(2,1)`?
- Non-goals: Do not re-audit the full radio switch, full `GetDockCoord` flag parser, refinery art footprint system, or Rust implementation beyond the narrow arrival surface.
- Evidence needed to mark COMPLETE: decompile plus assembly context for the PerCellProcess branch, evidence for the vtable `+0xA8` target and cell compare, and current Rust scan enough to hand off code/doc implications.
- Stop conditions: stop after the arrival branch is proven for mission/type/current-cell/destination-cell/radio-send relation, all report open questions are resolved/deferred, and this report is written.

## 1. Overview

`UnitClass__PerCellProcess @ 0x00739EC0` does call the destination building's vtable `+0xA8` during the Mission 7 / `0x19` dock-arrival path. It converts both the unit's current lepton coordinate and the returned dock coordinate to cells, compares cell X/Y equality, and only then sends radio `0x15` and stops/powers off the locomotor.

For stock GAREFN/NAREFN, the active `GetDockCoord` branch is the `Refinery=yes` branch at `BuildingType+0x16BB`, not the `Weeder=yes` branch at `+0x16BC`. On a stock 4x3 refinery placed at NW `(10,10)`, the `+0x16BB` branch still produces cell `(12,11)`, because it starts from `BuildingClass__GetCoords` foundation center and adds `+128` leptons in X.

## 2. Key Offsets And Slots

| Field / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit vtable `+0x184` | Current mission getter | `0x0073A324`, `0x0073A334`; checks `7` then `0x19` | Yes |
| Destination `WhatAmI` vtable `+0x2C` | Requires destination abstract type `6` building | `0x0073A343..0x0073A359` | Yes |
| Destination building vtable `+0xA8` | `BuildingClass__GetDockCoord` for buildings | `0x0073A391..0x0073A3B1`; vtable slot documented in `fn-building-getdockcoord.md:184` | Yes |
| BuildingType `+0x16B3` | `DockUnload=yes`, receiver `0x15` mission handoff | `0x0043C2D0` case `0x15`; `rulesmd.ini:[GAREFN]/[NAREFN]` | Yes |
| BuildingType `+0x16BB` | `Refinery=yes`, stock `GetDockCoord` branch | `0x00447B9E..0x00447BC4`; `rulesmd.ini:[GAREFN]/[NAREFN]` | Yes |
| BuildingType `+0x16BC` | `Weeder=yes`, first `GetDockCoord` branch | `0x00447B2D..0x00447B64`; flag map in `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md:55-57` | No for stock GAREFN/NAREFN |
| Unit vtable `+0x274` | Radio send to current contact | `0x0073A503..0x0073A507` sends `0x15` | Yes |
| Unit locomotor pointer `+0x674` | Locomotor stopped after radio `0x15` | `0x0073A50D..0x0073A52B` | Yes |

## 3. Core Logic

### 3.1 Arrival predicate in `0x00739EC0`

Verified active YR branch:

1. Current mission must be `7` or `0x19`.
2. `FootClass__GetDestination()` result must be non-null.
3. Destination `WhatAmI()` must equal `6` (building).
4. Unit vtable `+0x48` returns the unit's current lepton coords.
5. Destination building vtable `+0xA8` is called with the unit as requester.
6. Both coords are sign-correct shifted to cell space and compared by X/Y cell.
7. If equal, the function calls `FootClass__PerCellProcess(2)`, sends radio `0x15`, then calls locomotor vtable `+0x5C`.

**Evidence:** decompile `0x00739EC0`; assembly context `0x0073A324..0x0073A359`, `0x0073A369`, `0x0073A391..0x0073A3B1`, `0x0073A417..0x0073A437`, `0x0073A4F7..0x0073A52B`.  
**Active in YR:** Yes. This path is reached by standard harvester/refinery docking through Mission 7 / dock destination.

### 3.2 Stock refinery `GetDockCoord` result

`BuildingClass__GetDockCoord @ 0x00447B20` checks `+0x16BC` first and returns `NW+(2,1)` for Weeder. Stock GAREFN/NAREFN do not use that branch.

If `+0x16BC` is false and `+0x16BB` is true, the function calls the building's vtable `+0x48` coordinate getter and adds `+0x80` leptons to X. For a stock 4x3 building at NW `(10,10)`, `BuildingClass__GetCoords` is foundation center:

- center X = `10*256 + (4-1)*128 = 2944`
- center Y = `10*256 + (3-1)*128 = 2816`
- stock refinery dock X = `2944 + 128 = 3072`
- cell = `(3072 >> 8, 2816 >> 8) = (12,11)`

**Evidence:** decompile `0x00447B20`; assembly context `0x00447B2D..0x00447B64` for `+0x16BC`, `0x00447B9E..0x00447BC4` for `+0x16BB`; `BuildingClass__GetCoords @ 0x00447AC0` from coord-cell docs.  
**Active in YR:** Yes through `+0x16BB Refinery=yes` for stock GAREFN/NAREFN; No for `+0x16BC Weeder` on stock GAREFN/NAREFN.

### 3.3 Relation to accepted `0x0E` cell

`BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x0E` still computes the accepted `MOVE_TO_CELL(0x12)` payload as building NW `+(3,1)` for DockUnload/Weeder acceptance. That is a separate receiver-side admission target and does not call `GetDockCoord`.

Therefore the evidence supports a two-coordinate model for stock 4x3 refineries:

| Coordinate | Stock NW `(10,10)` | Binary source | Active in YR |
|---|---:|---|---|
| Accepted `0x0E` move cell | `(13,11)` | `0x0043C2D0` case `0x0E` hardcoded `Get_Cell_Packed + (3,1)` | Yes |
| PerCellProcess pad-arrival dock cell | `(12,11)` | `0x00739EC0` calls destination `+0xA8`; `0x00447B20` stock `+0x16BB` branch | Yes |
| Art `QueueingCell=4,1` | `(14,11)` | `artmd.ini` data, not accepted `0x0E` | Conditional |

**Evidence:** decompile `0x0043C2D0`; `CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md:31-48`; decompile `0x00739EC0`; decompile `0x00447B20`.  
**Active in YR:** Yes for both first two rows; conditional for QueueingCell fallback/waiting paths.

## 4. INI Keys

| File / section | Key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `rulesmd.ini:[CMIN]` | `Dock`, `Harvester` | `NAREFN,GAREFN`, `yes` | Allows CMIN to enter the standard harvester/refinery path | Yes |
| `rulesmd.ini:[HARV]` | `Dock`, `Harvester` | `NAREFN,GAREFN`, `yes` | Allows HARV to enter the same path | Yes |
| `rulesmd.ini:[GAREFN]` | `DockUnload`, `Refinery`, `NumberOfDocks` | `yes`, `yes`, `1` | Receiver handoff and stock `GetDockCoord` refinery branch | Yes |
| `rulesmd.ini:[NAREFN]` | `DockUnload`, `Refinery`, `NumberOfDocks` | `yes`, `yes`, `1` | Same as GAREFN | Yes |
| `artmd.ini:[GAREFN]` | `Foundation`, `QueueingCell`, `RemoveOccupy1` | `4x3`, `4,1`, `3,1` | Foundation controls `GetCoords`; QueueingCell is not accepted `0x0E`; RemoveOccupy opens `(13,11)` but does not change `GetDockCoord` | Yes / Conditional as noted |
| `artmd.ini:[NAREFN]` | `Foundation`, `QueueingCell`, `RemoveOccupy8` | `4x3`, `4,1`, `3,1` | Same cell relationship as GAREFN | Yes / Conditional as noted |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x0E` | Sends accepted `0x12` payload `NW+(3,1)` | decompile and prior accepted-anchor report | Yes |
| `UnitClass__PerCellProcess @ 0x00739EC0` | Later arrival cell equality gate and radio `0x15` sender | decompile plus assembly context | Yes |
| `BuildingClass__GetDockCoord @ 0x00447B20` | Supplies destination `+0xA8` dock coordinate | decompile plus assembly context | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15` | Receiver sets sender mission `0x10` for DockUnload | decompile and `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md` | Yes |
| `UnitClass__Receive_Radio @ 0x00737430` case `0x16` | Likely upstream timing-sync/approach bridge between accepted cell and pad cell | prior `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`; not re-decompiled in this slot | Yes, but deferred here |

## 6. Current Rust Implementation Status

| Surface | Current behavior | Status |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell` | Returns `(rx+3, ry+1)` for accepted `0x0E`-like cell | Matches accepted-anchor evidence |
| `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell` | Currently returns `(rx+2, ry+1)` with no DockingOffset | Matches stock 4x3 `GetDockCoord` cell result, but comment names this as a hardcoded retail refinery offset without distinguishing `+0x16BB` from `+0x16BC` |
| `phase_mission_enter` | Starts `Linked` when `snap` is at accepted cell `(rx+3, ry+1)` and not moving | Mismatch risk: gamemd's `0x15` arrival gate requires current cell equal destination `+0xA8` dock cell, which is `(rx+2, ry+1)` for stock 4x3 |
| `phase_linked` | Sets `snap.rx/snap.ry` to pad `(rx+2, ry+1)` but does not move entity position there | Mismatch risk: Rust can have internal dock state at `(12,11)` while entity remains physically at `(13,11)` |
| `miner_tests.rs::refinery_pad_and_conditional_release_cells` | Expects `(12,11)` for `refinery_pad_cell` | Correct for stock `GetDockCoord` cell, but test wording should not imply accepted `0x0E` cell |
| `miner_tests.rs::accepted_cell_arrival_sets_contact_entered_then_0x15_starts_unload_fsm` | Spawns miner at `(13,11)` and expects `Linked` | Likely drift for exact PerCellProcess arrival gate |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass__PerCellProcess @ 0x00739EC0` mission/building/dock-cell predicate | verified | decompile + assembly `0x0073A324..0x0073A437` | none |
| `UnitClass__PerCellProcess` radio `0x15` ordering | verified | assembly `0x0073A4F7..0x0073A52B` | none |
| `BuildingClass__GetDockCoord @ 0x00447B20` stock refinery `+0x16BB` branch | verified | decompile + assembly `0x00447B9E..0x00447BC4` | none for stock 4x3 cell result |
| `BuildingClass__GetDockCoord` `+0x16BC` branch | verified as not stock GAREFN/NAREFN | decompile + assembly `0x00447B2D..0x00447B64`; flag docs/INI | parser address not re-audited here |
| Accepted `0x0E` `NW+(3,1)` anchor | touched-not-exhausted | decompile `0x0043C2D0`; prior accepted-anchor report | full switch not re-audited |
| `UnitClass__Receive_Radio(0x16)` bridge | deferred | prior `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` | exact movement from `(13,11)` to `(12,11)` needs a targeted slot |
| Current Rust arrival transition | verified enough for delta | Codegraph and source scan | future patch/test required |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does PerCellProcess call destination vtable +0xA8? -> Yes, after mission 7/0x19 and destination building checks.` (evidence: `0x0073A391..0x0073A3B1`)
- `[RESOLVED] OQ-2 - Does it compare current cell to returned dock coordinate? -> Yes, both coords are shifted to cell space and X/Y compared before the arrival handoff.` (evidence: `0x0073A3B7..0x0073A437`)
- `[RESOLVED] OQ-3 - Does stock GAREFN/NAREFN take the +0x16BC NW+2 branch? -> No; stock refineries use `Refinery=yes`/`+0x16BB`, while `+0x16BC` is Weeder.` (evidence: `0x00447B2D..0x00447BA6`; `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md:55-57`; `rulesmd.ini`)
- `[RESOLVED] OQ-4 - Does stock +0x16BB still yield NW+(2,1) for 4x3 refineries? -> Yes, foundation center plus +128 X converts to cell `(rx+2, ry+1)` for 4x3.` (evidence: `0x00447B9E..0x00447BC4`; `fn-building-getcoords.md`)
- `[RESOLVED] OQ-5 - Does PerCellProcess send radio 0x15 only after the equality gate? -> Yes; `FootClass__PerCellProcess(2)` then radio `0x15`, then locomotor `+0x5C`.` (evidence: `0x0073A4F7..0x0073A52B`)
- `[RESOLVED] OQ-6 - Is accepted 0x0E NW+(3,1) the same as the PerCellProcess dock cell? -> No for stock 4x3 refineries: accepted anchor is `(rx+3,ry+1)`, dock-arrival coordinate is `(rx+2,ry+1)`.` (evidence: `0x0043C2D0`, `0x00447B20`, `0x00739EC0`)
- `[DEFERRED] OQ-7 - What exact upstream call moves/synchronizes the unit from accepted `(rx+3,ry+1)` to dock `(rx+2,ry+1)`?` (category: `out-of-scope`; reason: this slot was limited to PerCellProcess arrival and GetDockCoord relation; next-step-if-pursued: targeted audit of `UnitClass__Receive_Radio(0x16)` and locomotor target writes after `0x0E` accepted reply)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock `0x0E` accepted cell is `NW+(3,1)`, but PerCellProcess `0x15` arrival requires destination `+0xA8` dock cell, which is `NW+(2,1)` for 4x3 GAREFN/NAREFN | `0x0043C2D0`; `0x00739EC0`; `0x00447B20` | Rust currently transitions to `Linked` at accepted `(13,11)` then snaps only `snap.rx/snap.ry` to pad `(12,11)` | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`, `phase_linked` | Preserve accepted `(13,11)` as admission target, but do not fire the `0x15`/Linked handoff until the miner's physical/current cell is the dock coord `(12,11)` or until the verified `0x16` bridge says otherwise | `accepted_cell_then_getdockcoord_pad_arrival_before_0x15` | Do not collapse accepted cell and pad cell into one helper |
| `refinery_pad_cell(10,10,4,3,None) == (12,11)` is correct for stock `GetDockCoord` cell, but not because `+0x16BC` is stock refinery | `0x00447B9E..0x00447BC4`; `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md:55-57` | Test/comment wording currently implies hardcoded branch is the stock refinery mechanism | `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`; `miner_tests.rs::refinery_pad_and_conditional_release_cells` | Keep or implement `(12,11)` as stock pad/dock coord, but document it as the active `+0x16BB Refinery=yes` branch result for 4x3, not the `+0x16BC Weeder` branch | `stock_refinery_getdockcoord_4x3_uses_refinery_branch_pad_cell` | Do not claim `+0x16BC` is GAREFN/NAREFN parity evidence |
| `BuildingClass__Receive_Radio(0x15)` sets sender mission `0x10` for `DockUnload=yes`; PerCellProcess itself sends radio `0x15`, not mission/radio `0x10` | `0x0073A503..0x0073A507`; `0x0043C2D0` case `0x15` | Rust has phase transitions instead of mission IDs; broad behavior exists but frame/position gate is suspect | `src/sim/miner/miner_dock_sequence.rs` phase transitions | Gate the unload FSM on the verified pad-arrival event, not on accepted-cell arrival alone | `pad_arrival_radio_0x15_starts_unload_mission_0x10` | Do not start unload from the `0x0E` accepted cell if current cell is still `(13,11)` |

## Negative Facts / Do Not Do

- Do not state that stock GAREFN/NAREFN take `BuildingType+0x16BC`; that is the Weeder branch, not stock refinery.
- Do not state that `NW+(2,1)` is Weeder-only; stock 4x3 refineries also produce `(rx+2,ry+1)` through the `+0x16BB Refinery=yes` branch.
- Do not replace accepted `0x0E` `NW+(3,1)` with `NW+(2,1)`; these are different stages.
- Do not treat `QueueingCell=4,1` as either the accepted `0x0E` cell or the PerCellProcess dock cell.
- Do not let Rust internal `snap.rx/snap.ry` drift from entity physical position without an explicit, verified gamemd-equivalent position/locomotion event.

## Stale Docs / Follow-up Docs

- `docs/research/coord-cell-conversions/fn-building-getdockcoord.md` replacement wording:
  - Replace "Refinery pad branch (`BuildingTypeClass+0x16bc != 0`)" with "`Weeder=yes` branch (`BuildingTypeClass+0x16BC != 0`): returns `NW+(2,1)`."
  - Replace "Only branch 1 is active for refineries in standard YR" with "For stock GAREFN/NAREFN, branch 1 is not active; the stock refinery path uses `Refinery=yes` at `+0x16BB`, which for 4x3 foundations also converts to cell `NW+(2,1)`."
  - Replace branch 2 wording "GetCoords of requester" with "GetCoords of the building object via the helper's `ECX=this` dispatch; the requester argument is present at the call site but is not the coordinate source for the stock `+0x16BB` result."
- `docs/research/coord-cell-conversions/_parity.md` row 35 replacement wording:
  - "`BuildingClass__GetDockCoord` stock refinery branch is active through `BuildingTypeClass+0x16BB Refinery=yes`, not `+0x16BC`. For 4x3 GAREFN/NAREFN at NW `(10,10)`, it returns dock cell `(12,11)`. This does not replace the accepted `CAN_DOCK(0x0E)` move target `(13,11)`; it is the later PerCellProcess pad-arrival coordinate used before radio `0x15`."
- `docs/research/coord-cell-conversions/_system.md` replacement wording:
  - "Refinery dock pad" should distinguish: accepted `0x0E` target `NW+(3,1)`, stock `GetDockCoord`/PerCellProcess dock-arrival coordinate `NW+(2,1)`, and QueueingCell/wait target `NW+(4,1)`.

## Sources

- Ghidra `decompile_function 0x00739EC0`
- Ghidra `get_assembly_context` around `0x0073A324`, `0x0073A369`, `0x0073A3B1`, `0x0073A417`, `0x0073A4F7`, `0x0073A503`, `0x0073A52B`
- Ghidra `decompile_function 0x00447B20`
- Ghidra `get_assembly_context` around `0x00447B2D`, `0x00447B35`, `0x00447B64`, `0x00447B9E`, `0x00447BA6`
- Ghidra `decompile_function 0x0043C2D0`
- Ghidra `decompile_function 0x005F6C80`; `get_assembly_context 0x005F6C80`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `docs/research/miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`
- `docs/research/miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/coord-cell-conversions/fn-building-getdockcoord.md`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_tests.rs`
