# Building Receive Radio 0x0E Stock Refinery CAN_DOCK Cell - Ghidra Research Report

**Address(es):** `0x0043C2D0` (`BuildingClass__Receive_Radio`), `0x0041BEA0` (`ObjectClass__Get_Cell_Packed`), `0x004D8FB0` (`FootClass__Receive_Radio`), `0x00447B20` (`BuildingClass__GetDockCoord`), `0x00739EC0` (`UnitClass__PerCellProcess`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR `GAREFN`/`NAREFN` `DockUnload=yes` receiver-side `CAN_DOCK(0x0E)` admission in `BuildingClass::Receive_Radio`, including branch predicates, `MOVE_TO_CELL(0x12)` payload cell, and negative proof for `GetDockCoord`, `QueueingCell`, and `DockingOffset` reads in that accepted path.  
**Non-Scope:** full far-return `QueueingCell` staging, full unload/departure FSM, every non-refinery dock class, exact two-miner same-frame runtime ordering, and implementation changes.  
**Confidence:** High for the stock accepted cell and negative input set.  
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN`.

> **Correction 2026-05-24 - `GetDockCoord` flag wording**
>
> The accepted stock refinery `0x0E` / `0x12` target is still `NW+(3,1)` and
> this accepted path still does not call `GetDockCoord`. However, wording below
> that associates `NW+(2,1)` only with the `+0x16BC` Weeder branch is stale.
> Stock GAREFN/NAREFN also produce `GetDockCoord == NW+(2,1)` through the
> active `+0x16BB Refinery=yes` branch. Keep these as separate frames:
> accepted `0x12` target `NW+(3,1)`, `GetDockCoord` coordinate `NW+(2,1)`,
> and `QueueingCell=4,1` staging `NW+(4,1)`.

## 0. Investigation Contract

**Target question:** For standard YR GAREFN/NAREFN `DockUnload=yes` admission, what exact cell does `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` send as `MOVE_TO_CELL(0x12)`, and does that path consume `GetDockCoord`, `QueueingCell`, or `DockingOffset`?  
**Non-goals:** Do not implement Rust, do not patch coord-cell docs in this slot, do not re-investigate far-return staging or post-unload exit beyond separating them from accepted `0x0E`.  
**Evidence needed to mark COMPLETE:** decompile plus executable disassembly range for the case `0x0E` payload; decompile for `Get_Cell_Packed` and `FootClass` `0x12` payload handling; stock INI proof for `DockUnload`/`Refinery`/harvester liveness; current Rust surface scan; stale-doc replacement wording.  
**Stop conditions:** Stop after the accepted stock branch is resolved, all direct input alternatives are positively excluded, and all open questions are resolved or explicitly deferred.

## 1. Overview

`BuildingClass__Receive_Radio` case `0x0E` is the receiver-side `CAN_DOCK` admission branch. For stock `GAREFN` and `NAREFN`, once the branch reaches the accepted `DockUnload || Weeder` block, it computes the payload cell from the building packed/NW cell plus `(3,1)` and sends that `CellClass*` to the miner with radio `0x12`.

The accepted stock refinery target is therefore `NW+(3,1)`, not `NW+(2,1)`. `NW+(2,1)` belongs to the `BuildingClass::GetDockCoord` `+0x16BC` branch, and this accepted `0x0E` code path does not call `GetDockCoord`.

## 2. Key Offsets / Flags

| Owner | Offset / slot | Verified meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x16B3` | `DockUnload=yes`; enables stock refinery admission and `0x15` unload handoff | Yes for `GAREFN`/`NAREFN` | `0x0043C2D0`; `rulesmd.ini:11726`, `12519` |
| `BuildingTypeClass` | `+0x16BB` | `Refinery=yes`; separate refinery type flag, not the `0x0E` accepted-cell predicate | Yes for `GAREFN`/`NAREFN` | `0x00447B20`; `rulesmd.ini:11727`, `12520` |
| `BuildingTypeClass` | `+0x16BC` | `Weeder=yes`; shares accepted `0x0E` `(3,1)` branch, but is not stock GAREFN/NAREFN | No for stock GAREFN/NAREFN | `0x0043C2D0`; `rulesmd.ini` stock sections lack `Weeder=yes` |
| `BuildingTypeClass` | `+0x1618/+0x161C` | parsed `QueueingCell`, not read by accepted `0x0E` block | Conditional elsewhere; No for accepted anchor input | no load in `0x0043C2D0` accepted block; `artmd.ini:1716`, `1773` |
| `BuildingTypeClass` | `+0x1780/+0x1788` | `NumberOfDocks` / `DockingOffset[]`, consumed by `GetDockCoord` dock-offset branches | Conditional elsewhere; No for stock accepted anchor | `0x00447B20`; no accepted-block read in `0x0043C2D0` |
| `ObjectClass` | `+0x9C/+0xA0` | object world X/Y converted to packed cell with sign-correct `/256` | Yes | `0x0041BEA0` |

## 3. Core Logic

### 3.1 Case `0x0E` admission skeleton

Verified behavior:

1. `BuildingClass__Receive_Radio @ 0x0043C2D0` dispatches case `0x0E`.
2. It first calls base `TechnoClass__Receive_Radio(sender, 0x0E, param)` and then rejects unpowered buildings with return `10`.
3. It runs special rejection/eligibility gates for repair/bunker and contact capacity paths.
4. On the standard non-helipad/non-alternate-dock branch, it verifies/creates contact state, sends/checks `0x13`, then writes `*param_4 = this`.
5. If neither `DockUnload` nor `Weeder` is true, it takes unrelated non-stock/helipad branches.
6. If `DockUnload || Weeder` is true, it computes a packed cell from vtable `+0x1B8`, adds `x+3` and `y+1`, converts that cell through `MapClass__Get_CellClass`, writes the resulting `CellClass*` to `*param_4`, and sends `0x12` through vtable `+0x27C`.
7. Only if the unit returns `0x14` from `0x12` does the building send directed `0x18` and then directed `0x16`.

**Active in YR:** Yes. `GAREFN` and `NAREFN` both set `DockUnload=yes`; `CMIN` and `HARV` both target `NAREFN,GAREFN`.

**Evidence:** decompile `0x0043C2D0`; executable range checked with `disassemble_bytes 0x0043C4D0..0x0043C90B`; stock INI lines `rulesmd.ini:7361`, `7364`, `8225`, `8228`, `11726`, `12519`.

### 3.2 Exact payload cell

The accepted stock branch does:

```text
packed = building.vtable+0x1B8()
move_cell.x = packed.x + 3
move_cell.y = packed.y + 1
payload = MapClass::Get_CellClass(move_cell)
send radio 0x12 with payload CellClass*
```

For a refinery whose packed/NW cell is `(10,10)`, the accepted `0x12` target is `(13,11)`.

**Active in YR:** Yes for stock `GAREFN`/`NAREFN`, because `DockUnload=yes` is true and the standard refinery branch reaches the `DockUnload || Weeder` block.  
**Evidence:** decompile `0x0043C2D0`; disassembly context range `0x0043C650..0x0043C90B`; `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` confirms packed cell is derived from object X/Y by sign-correct shift by 8.

### 3.3 Payload shape and response contract

`FootClass__Receive_Radio @ 0x004D8FB0` case `0x12` treats `*param_4` as a `CellClass*`. If the foot object's current packed cell equals the payload cell's `+0x48` coordinate converted to cell space, it returns `0x14`. Otherwise it sets destination through vtable `+0x480`, stamps current frame fields, and returns `1`.

This proves the building's `0x12` payload is not a raw `(x,y)` pair; it is a `CellClass*` obtained from `MapClass__Get_CellClass`.

**Active in YR:** Yes for units receiving building-directed `MOVE_TO_CELL(0x12)`.  
**Evidence:** decompile `0x004D8FB0`; radio helper `0x0065A970` forwards non-HELLO/non-BREAK messages synchronously to target `Receive_Radio`.

### 3.4 Negative input proof

The accepted `0x0E` block does not read:

- `QueueingCell` storage at `BuildingTypeClass+0x1618/+0x161C`.
- `DockingOffset[]` storage at `BuildingTypeClass+0x1788`.
- `NumberOfDocks` as a coordinate selector.
- `BuildingClass::GetDockCoord @ 0x00447B20`.
- foundation width/height.
- refinery art ID (`GAREFN` vs `NAREFN`) after the common `DockUnload` branch predicate.

`BuildingClass::GetDockCoord` is a real function and its first branch for `+0x16BC` returns centered lepton coordinates at packed `+(2,1)`, but that branch is not invoked by the standard accepted `CAN_DOCK(0x0E)` payload computation.

**Active in YR:** Negative fact is active for stock `GAREFN`/`NAREFN` accepted admission. `GetDockCoord` remains active elsewhere/conditioned by its callers.  
**Evidence:** decompile `0x0043C2D0`; decompile `0x00447B20`; `disassemble_bytes 0x00447B20..0x00447D13`; `artmd.ini:1716`, `1773`, `1725`.

### 3.5 Later arrival is separate

`UnitClass__PerCellProcess @ 0x00739EC0` has a later arrival branch that compares the unit's current cell with the destination building's vtable `+0xA8` dock coordinate, then sends radio `0x15`. That is not the same computation as the accepted `0x0E` target.

For stock refinery art, the accepted target `(NW+3,NW+1)` is also opened by `RemoveOccupy`:

- `NAREFN`: `RemoveOccupy8=3,1` at `artmd.ini:1760`.
- `GAREFN`: `RemoveOccupy1=3,1` at `artmd.ini:1795`.

**Active in YR:** Yes for the stock physical pad/passable-cell relationship, but the exact `GetDockCoord` later-arrival consumer is slot 4's scope.  
**Evidence:** decompile `0x00739EC0`; stock art INI lines above.

## 4. INI Keys

| File | Section | Key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|---|---|
| `rulesmd.ini` | `[CMIN]` | `Dock` | `NAREFN,GAREFN` | Lets CMIN target stock refineries | Yes (`7361`) |
| `rulesmd.ini` | `[CMIN]` | `Harvester` | `yes` | Puts CMIN on harvester/refinery path | Yes (`7364`) |
| `rulesmd.ini` | `[HARV]` | `Dock` | `NAREFN,GAREFN` | Lets HARV target stock refineries | Yes (`8225`) |
| `rulesmd.ini` | `[HARV]` | `Harvester` | `yes` | Puts HARV on harvester/refinery path | Yes (`8228`) |
| `rulesmd.ini` | `[GAREFN]` | `DockUnload` | `yes` | Selects accepted `0x0E` stock branch | Yes (`11726`) |
| `rulesmd.ini` | `[GAREFN]` | `Refinery` | `yes` | Refinery behavior elsewhere; not the accepted-cell predicate | Yes (`11727`) |
| `rulesmd.ini` | `[NAREFN]` | `DockUnload` | `yes` | Selects accepted `0x0E` stock branch | Yes (`12519`) |
| `rulesmd.ini` | `[NAREFN]` | `Refinery` | `yes` | Refinery behavior elsewhere; not the accepted-cell predicate | Yes (`12520`) |
| `artmd.ini` | `[GAREFN]` | `QueueingCell` | `4,1` | not read by accepted `0x0E`; waiting/far-staging data | Conditional elsewhere (`1773`) |
| `artmd.ini` | `[NAREFN]` | `QueueingCell` | `4,1` | not read by accepted `0x0E`; waiting/far-staging data | Conditional elsewhere (`1716`) |
| `artmd.ini` | `[GAREFN]` | `RemoveOccupy1` | `3,1` | opens accepted pad cell | Yes (`1795`) |
| `artmd.ini` | `[NAREFN]` | `RemoveOccupy8` | `3,1` | opens accepted pad cell | Yes (`1760`) |
| `artmd.ini` | `[NAREFN]` | `DockingOffset0` | commented `256,0,0` | inactive and not read by accepted `0x0E` | No (`1725`) |

## 5. Integration Points

| Function | Role | Active in YR | Evidence |
|---|---|---|---|
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Owner of receiver-side `CAN_DOCK(0x0E)` accepted payload | Yes | decompile; disassembly range `0x0043C4D0..0x0043C90B` |
| `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` | packed/NW cell source via sign-correct shift | Yes | decompile |
| `MapClass__Get_CellClass` | converts `(NW+3,NW+1)` cell to `CellClass*` payload | Yes | call in `0x0043C2D0` accepted branch |
| `RadioClass__Transmit_Radio_Impl @ 0x0065A970` | forwards `0x12` synchronously to target receiver | Yes | decompile |
| `FootClass__Receive_Radio @ 0x004D8FB0` | consumes `0x12` `CellClass*`, returns `0x14` if already there | Yes | decompile |
| `BuildingClass__GetDockCoord @ 0x00447B20` | separate dock-coordinate helper; not called for accepted `0x0E` payload | Conditional elsewhere | decompile |
| `UnitClass__PerCellProcess @ 0x00739EC0` | later physical arrival sends `0x15` | Yes | decompile; detailed scope deferred to slot 4 |

## 6. Current Rust Implementation Status

No Rust files were modified by this investigation.

| Rust surface | Current observed status |
|---|---|
| `src/sim/miner/miner_dock_sequence.rs:100` `refinery_can_dock_queue_cell` | Matches this report: returns `(rx+3, ry+1)` and comments that `QueueingCell` is not read. |
| `src/sim/miner/miner_system.rs:1243` `refinery_dock_cell` | Delegates to `refinery_can_dock_queue_cell`, so accepted stock target remains `(3,1)`. |
| `src/sim/miner/miner_dock_sequence.rs:108` `refinery_pad_cell` | Current dirty change claims a retail fallback `(+2,+1)`. This report does not validate using that as the stock accepted admission cell. |
| `src/sim/miner/miner_tests.rs:1983` | Current dirty test expects `refinery_pad_cell(10,10,4,3,None) == (12,11)`. That expectation should not be used to replace the accepted `CAN_DOCK` cell. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__Receive_Radio` case `0x0E` stock `DockUnload` accepted payload | verified | `0x0043C2D0`; `0x0043C650..0x0043C90B` | none for accepted cell |
| `DockUnload` stock liveness | verified | `rulesmd.ini:11726`, `12519`; `CMIN/HARV Dock=` lines | none |
| exact payload cell `NW+(3,1)` | verified | `0x0043C2D0`; `0x0041BEA0` | none |
| payload pointer shape as `CellClass*` | verified | `0x0043C2D0`; `0x004D8FB0` | none |
| `QueueingCell` negative for accepted `0x0E` | verified | no `+0x1618/+0x161C` load in accepted block; `artmd.ini` context | far/wait staging out-of-scope |
| `DockingOffset` negative for accepted `0x0E` | verified | no `+0x1788` load in accepted block; `0x00447B20` separate consumer | slot 2/4 cover later consumers |
| `GetDockCoord` `NW+(2,1)` separation | verified | `0x00447B20`; no call in accepted block | exact later-arrival impact delegated to slot 4 |
| stock pad art corroboration | verified | `artmd.ini:1760`, `1795` | slot 5 may expand footprint/passability |
| Rust surface scan | verified structurally | `src/sim/miner/*` grep/read | tests not run in this read-only research slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What mode applies? -> exhaustive-slice for one receiver case and one stock branch.` (evidence: assignment scope)
- `[RESOLVED] OQ-02 - Does an existing exact report already cover this? -> prior miner reports cover it, but this swarm slot re-verifies due stale coord-cell contradiction and writes the assigned report.` (evidence: doc search; this report path did not exist before write)
- `[RESOLVED] OQ-03 - What branch predicate enables stock accepted refinery admission? -> `Type+0x16B3 DockUnload` true, with shared `DockUnload || Weeder` accepted block.` (evidence: `0x0043C2D0`; `rulesmd.ini:11726`, `12519`)
- `[RESOLVED] OQ-04 - What exact cell is sent? -> packed/NW building cell plus `(3,1)`.` (evidence: `0x0043C2D0`, `0x0041BEA0`)
- `[RESOLVED] OQ-05 - Is the payload raw coords or a cell pointer? -> `CellClass*` from `MapClass__Get_CellClass`.` (evidence: `0x0043C2D0`, `0x004D8FB0`)
- `[RESOLVED] OQ-06 - Does accepted `0x0E` read `QueueingCell`? -> No.` (evidence: `0x0043C2D0`; `QueueingCell` only INI context)
- `[RESOLVED] OQ-07 - Does accepted `0x0E` read `DockingOffset` or `NumberOfDocks` for the payload? -> No.` (evidence: `0x0043C2D0`; `0x00447B20` separate consumer)
- `[RESOLVED] OQ-08 - Does accepted `0x0E` call `GetDockCoord`? -> No; it calls packed-cell and map-cell helpers instead.` (evidence: `0x0043C2D0`, `0x00447B20`)
- `[RESOLVED] OQ-09 - Is `NW+(2,1)` active for stock accepted GAREFN/NAREFN admission? -> No.` (evidence: `0x0043C2D0`; `0x00447B20` `+0x16BC` branch; stock INI lacks `Weeder=yes`)
- `[RESOLVED] OQ-10 - Do GAREFN and NAREFN differ in this branch? -> No; both enter the same `DockUnload=yes` branch.` (evidence: `rulesmd.ini:11726`, `12519`)
- `[RESOLVED] OQ-11 - Does `FootClass` return already-there before 0x18/0x16? -> Yes, `0x12` returns `0x14` only if current cell equals payload cell.` (evidence: `0x004D8FB0`)
- `[RESOLVED] OQ-12 - Does stock art corroborate the accepted cell? -> Yes, both stock refinery arts remove occupancy at `(3,1)`.` (evidence: `artmd.ini:1760`, `1795`)
- `[RESOLVED] OQ-13 - What current Rust surface is implicated? -> accepted helper matches `(3,1)`; dirty pad helper/test now assert `(2,1)` and must not replace accepted target.` (evidence: `src/sim/miner/miner_dock_sequence.rs:100`, `108`; `miner_tests.rs:1983`)
- `[DEFERRED] OQ-14 - Does later `PerCellProcess` consume `GetDockCoord` in a way that should redefine Rust pad state?` (category: `out-of-scope`; reason: assigned to slot 4; next-step-if-pursued: reconcile `0x00739EC0` vtable `+0xA8` against stock refinery art)
- `[DEFERRED] OQ-15 - Full far-return/wait `QueueingCell` path.` (category: `out-of-scope`; reason: not this slot's target and already covered by miner far-return reports; next-step-if-pursued: verify `Mission_Harvest` state 2 only)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock `DockUnload` `CAN_DOCK(0x0E)` sends `MOVE_TO_CELL(0x12)` to building NW `+(3,1)` as a `CellClass*`. | `0x0043C2D0`; `0x0041BEA0`; `0x004D8FB0`; `rulesmd.ini:11726`, `12519` | none observed for accepted helper; dirty pad helper is separate and suspicious if used for accepted cell | `src/sim/miner/miner_dock_sequence.rs::refinery_can_dock_queue_cell`; `src/sim/miner/miner_system.rs::refinery_dock_cell` | Preserve `(rx+3, ry+1)` for accepted stock refinery admission. | Proposed test: `stock_refinery_can_dock_move_to_cell_uses_nw_plus_3_1` | Do not replace accepted target with `GetDockCoord` `NW+(2,1)` or art `QueueingCell=4,1`. |
| Accepted `0x0E` does not read `QueueingCell`, `DockingOffset`, `NumberOfDocks`, foundation dimensions, or `GetDockCoord`. | `0x0043C2D0`; negative contrast `0x00447B20`; `artmd.ini:1716`, `1773` | current Rust mostly preserves split; dirty `refinery_pad_cell` doc says retail `(+2,+1)` and may mislead | `src/sim/miner/miner_dock_sequence.rs::resolve_refinery_cells`; tests around accepted vs pad/wait cells | Keep wait/far staging and physical/later pad concepts separate from accepted `0x0E`. | Proposed test: `refinery_candock_ignores_queueingcell_and_dockingoffset` | Do not make `QueueingCell`, `DockingOffset0`, or foundation width affect accepted stock admission. |
| Stale coord-cell row 35 should not be treated as proof that every miner deposit uses `NW+(2,1)`. | `0x0043C2D0`; `0x00447B20`; coord-cell doc's own unverified flag note | doc delta, not Rust delta by itself | `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_parity.md`; `fn-building-getdockcoord.md`; `_system.md` | Reword row/fix-list so `GetDockCoord +0x16BC` is not standard refinery accepted admission. | Proposed test: `refinery_pad_doc_contradiction_keeps_candock_cell_at_13_11` | Do not mark `NW+3 -> NW+2` as a fixed stock miner-deposit parity item. |

## 10. Negative Facts / Do Not Do

- Do not change stock accepted `CAN_DOCK` from `NW+(3,1)` to `NW+(2,1)`.
- Do not treat `BuildingClass::GetDockCoord` as the source of the accepted `0x0E` `MOVE_TO_CELL` payload.
- Do not use `QueueingCell=4,1` as the accepted `0x0E` target; it is waiting/far-staging data.
- Do not use `DockingOffset0` or `NumberOfDocks` to compute the stock GAREFN/NAREFN accepted `0x0E` target.
- Do not infer `+0x16BC` means stock refinery; stock GAREFN/NAREFN standard admission is proven by `+0x16B3 DockUnload`.

## 11. Remaining Uncertainty

- Exact later `PerCellProcess` `GetDockCoord`/vtable `+0xA8` interaction is intentionally left to swarm slot 4.
- Full stock art footprint/passability proof is intentionally left to swarm slot 5.
- Runtime frame ordering for two-miner contention is not covered here.

## 12. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/fn-building-getdockcoord.md`: replace "Only branch 1 is active for refineries in standard YR" with "Branch `Type+0x16BC` returns packed `NW+(2,1)` centered lepton coordinates, but this slot did not prove it is stock `GAREFN/NAREFN` refinery admission; stock accepted `BuildingClass::Receive_Radio(0x0E)` uses `DockUnload` and sends `NW+(3,1)` without calling `GetDockCoord`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_parity.md`: replace row 35's stock-refinery FIXED wording with "DRIFT/STALE: `GetDockCoord` `NW+(2,1)` is not the standard accepted stock refinery `CAN_DOCK` target; `Receive_Radio(0x0E)` sends `NW+(3,1)` for `DockUnload=yes`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/coord-cell-conversions/_system.md`: replace "Refinery dock pad - NW+3 -> NW+2 (every miner deposit)" with "Reconcile stale GetDockCoord/refinery-pad claim: stock accepted miner admission remains `NW+(3,1)`; `NW+(2,1)` is a separate `GetDockCoord +0x16BC` branch until proven live for a stock consumer."

## Sources

- Ghidra decompile: `BuildingClass__Receive_Radio @ 0x0043C2D0`.
- Ghidra disassembly range checks: `0x0043C4D0..0x0043C90B`, `0x0043C650..0x0043C90B`.
- Ghidra decompile: `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`.
- Ghidra decompile: `FootClass__Receive_Radio @ 0x004D8FB0`.
- Ghidra decompile: `RadioClass__Transmit_Radio_Impl @ 0x0065A970`.
- Ghidra decompile: `BuildingClass__GetDockCoord @ 0x00447B20`.
- Ghidra decompile: `UnitClass__PerCellProcess @ 0x00739EC0`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`.
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs`.
