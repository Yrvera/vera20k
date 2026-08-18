# Stock Refinery Art RemoveOccupy Pad Cell - Ghidra Research Report

**Address(es):** `0x0045FE50`, `0x00441F60`, `0x005683C0`, `0x0073F0A0`, `0x00458A00`, `0x00447B20`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR `GAREFN`/`NAREFN` art/rules occupancy and passability relation for refinery pad candidate cells `NW+(3,1)`, `NW+(2,1)`, and `QueueingCell=NW+(4,1)`.  
**Non-Scope:** full docking radio FSM, miner mission retry timing, refinery unload/deposit state machine, war factory/depot docking, and non-stock/modded refinery behavior beyond the named stock keys.  
**Confidence:** High for stock INI values, parser offsets, base foundation vs hidden-occupancy split, and checked passability relation; Medium for current Rust delta because only the relevant surfaces were scanned.  
**Active in YR:** Yes for stock `GAREFN`/`NAREFN`; conditional branches are gated by `DockUnload=yes`, `Bib=yes`, `NumberImpassableRows=3`, `CanHideThings=true`, and building contact state as noted below.

## 0. Working Notes

Target question: Which stock YR refinery art/rules cell is opened by `RemoveOccupy`/footprint passability, and does it align with `NW+(3,1)`, `NW+(2,1)`, or both?  
Non-goals: Do not re-investigate full docking radio, miner unload FSM, or all `GetDockCoord` branches except where needed to identify stale wording.  
Evidence needed to mark COMPLETE: exact stock INI lines, parser/binary mapping for relevant keys, binary evidence separating base foundation from hidden occupancy, binary evidence for passability of `(3,1)` vs `(2,1)`, Rust/doc handoff.  
Stop conditions: stop after stock `GAREFN`/`NAREFN` art passability relation and stale-doc replacement wording are resolved; defer full radio arrival routing to slots 2-4.

## 1. Overview

Stock YR Allied and Soviet refineries both use a `4x3` base foundation. The stock visible/passable harvester pad is the east-edge foundation cell `NW+(3,1)`, not `NW+(2,1)`. `QueueingCell=4,1` is one cell farther east and is a waiting/fallback cell outside the foundation.

`RemoveOccupy` participates in hidden occupancy (`CellClass+0x100`) behind `CanHideThings`; it does not remove the building object from the real `CellClass+0xE4` foundation list. The reason `NW+(3,1)` is enterable is the live `UnitClass::Can_Enter_Cell` building skip stack: stock `Bib=yes` opens the east edge, and contacted `NumberImpassableRows=3` also allows x-offset 3 while preserving x-offset 2 as blocked.

## 2. Key Offsets / Fields

| Field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `BuildingTypeClass+0x1570` | `Bib=yes` / HasBib | `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; `Can_Enter_Cell @ 0x0073F0A0` reads it in the east-neighbor skip | Yes: `rulesmd.ini:11730`, `12523` |
| `BuildingTypeClass+0x1620` | `NumberImpassableRows` | `0x0045FE50` reads `NumberImpassableRows`; helper `0x00458A00` compares cell X against origin X + value | Yes: `rulesmd.ini:11764`, `12524` |
| `BuildingTypeClass+0x1624..0x1660` | `AddOccupy1..8` | parser/string `AddOccupy%d` in `0x0045FE50`; hidden writer `0x005683C0` | Conditional: only hidden occupancy when `CanHideThings` |
| `BuildingTypeClass+0x1664..0x16A0` | `RemoveOccupy1..8` | parser/string `RemoveOccupy%d` in `0x0045FE50`; hidden writer `0x005683C0` decrements `Cell+0x100` if nonzero | Conditional: only hidden occupancy when `CanHideThings` |
| `BuildingTypeClass+0x16B3` | `DockUnload=yes` | `0x0045FE50` reads string `DockUnload`; stock refineries set it | Yes: `rulesmd.ini:11726`, `12519` |
| `BuildingTypeClass+0x16BB` | `Refinery=yes` | `0x0045FE50` reads string `Refinery`; stock refineries set it | Yes: `rulesmd.ini:11727`, `12520` |
| `BuildingTypeClass+0x16BC` | `Weeder=yes`, not stock refinery | `0x0045FE50` reads string `Weeder` into `+0x16BC`; stock refinery INI does not set `Weeder=yes` | No for stock `GAREFN`/`NAREFN`; conditional for Weeder types/mods |
| `CellClass+0xE4` | real ground object list scanned by `Can_Enter_Cell` | base foundation enter/place paths `0x00441F60`, `0x005683C0`; scan at `0x0073F0A0` | Yes |
| `CellClass+0x100` | hidden occupancy counter | hidden writer `0x005683C0`; not the normal movement object list | Conditional on `CanHideThings` |

## 3. Stock INI Facts

| Object | Stock values | Evidence | Active in YR |
|---|---|---|---|
| `GAREFN` rules | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `Bib=yes`, `NumberImpassableRows=3` | `ini/rulesmd.ini:11722`, `11726`, `11727`, `11729`, `11730`, `11764` | Yes |
| `NAREFN` rules | `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `Bib=yes`, `NumberImpassableRows=3` | `ini/rulesmd.ini:12515`, `12519`, `12520`, `12521`, `12523`, `12524` | Yes |
| `GAREFN` art | `Foundation=4x3`, `QueueingCell=4,1`, `CanHideThings=True`, `OccupyHeight=2`, `AddOccupy1=-1,0`, `AddOccupy2=-1,-1`, `RemoveOccupy1=3,1` | `ini/artmd.ini:1763`, `1766`, `1773`, `1790`, `1792`, `1793`, `1794`, `1795` | Yes |
| `NAREFN` art | `Foundation=4x3`, `QueueingCell=4,1`, `CanHideThings=true`, `OccupyHeight=4`, `RemoveOccupy1..8`, including `RemoveOccupy8=3,1` | `ini/artmd.ini:1706`, `1709`, `1716`, `1750`, `1752`, `1753..1760` | Yes |

Material finding: both stock refineries name `RemoveOccupy` offset `(3,1)`. Neither stock refinery has an art/rules key that opens or identifies `NW+(2,1)` as the harvester pad. Active in YR: Yes, because these are active `rulesmd.ini`/`artmd.ini` stock definitions and the parser paths above read these keys.

## 4. Core Logic

### 4.1 Base foundation remains 4x3

`BuildingClass::Place_OccupyMap @ 0x00441F60` walks the foundation cell list returned through the building foundation vtable and marks each base foundation cell. The decompiled path does not read `AddOccupy` or `RemoveOccupy` while placing normal building occupancy. Active in YR: Yes, this is standard building placement.

`TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` first adds the object to every base foundation cell's content list before the hidden-occupancy section. For stock `Foundation=4x3`, both `NW+(2,1)` and `NW+(3,1)` are in the real base object-list footprint. Active in YR: Yes for placed stock refineries.

### 4.2 RemoveOccupy is hidden occupancy, not real passability footprint

After base foundation object-list insertion, `0x005683C0` checks building type and `CanHideThings` at `+0x1766`. It then applies `OccupyHeight`, `AddOccupy`, and `RemoveOccupy` to `CellClass+0x100`. `RemoveOccupy` decrements the hidden counter if nonzero; it does not remove the building from `CellClass+0xE4`.

Active in YR: Conditional. Stock `GAREFN`/`NAREFN` set or retain `CanHideThings=true`, so their hidden occupancy modifiers are live. The effect is hidden-object/behind behavior, not ordinary movement object-list removal.

### 4.3 Why `NW+(3,1)` is passable

`UnitClass::Can_Enter_Cell @ 0x0073F0A0` scans objects in the candidate cell. For buildings, the stock refinery can stop blocking at the east edge via `Bib=yes`: the branch reads `+0x1570`, probes the east neighbor using initialized offset `(dx=+1, dy=0)`, and if the east neighbor does not contain the same building it skips the building as a blocker for that cell.

For a 4x3 foundation at origin `(rx,ry)`, every `x=rx+3` cell has an east neighbor outside the same building. Therefore `NW+(3,0)`, `NW+(3,1)`, and `NW+(3,2)` are relaxed by the stock bib branch. `NW+(2,1)` has east neighbor `NW+(3,1)`, which still contains the same building object, so the bib branch does not open it.

Active in YR: Yes for stock `GAREFN`/`NAREFN`, because they set `Bib=yes` in `rulesmd.ini`.

### 4.4 Contacted `NumberImpassableRows=3` agrees with the same boundary

`FUN_00458A00 @ 0x00458A00` first verifies the candidate cell's building is the target building, then returns true while `cell_x < building_origin_x + NumberImpassableRows`. In `Can_Enter_Cell`, a contacted building is skipped only when this helper returns false. For stock `NumberImpassableRows=3`, the contacted skip begins at x-offset 3 and does not apply to x-offset 2.

Active in YR: Conditional. The branch requires the moving unit's radio/contact vector to contain the building. It is live for docking/contact flows, but this report does not re-prove the entire radio FSM.

### 4.5 `GetDockCoord +0x16BC` is not stock refinery art pad evidence

`BuildingClass__GetDockCoord @ 0x00447B20` has a branch that reads `BuildingTypeClass+0x16BC` and returns `NW+(2,1)` in leptons. However `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` maps `+0x16BC` to the `Weeder` key, while stock `GAREFN`/`NAREFN` set `DockUnload=yes` and `Refinery=yes`, not `Weeder=yes`.

Active in YR: No for stock `GAREFN`/`NAREFN`; conditional for any type/mod that sets `Weeder=yes`.

## 5. Checked Cell Matrix

Assume stock `GAREFN`/`NAREFN` at NW `(10,10)`.

| Relative cell | Absolute cell | In base `4x3` foundation? | Stock art/rules relation | Net refinery blocker relation | Active in YR |
|---|---:|---|---|---|---|
| `NW+(2,1)` | `(12,11)` | Yes | no `RemoveOccupy`; west/interior protected by `NumberImpassableRows=3`; east neighbor is same building | Remains blocked by refinery building object | Yes |
| `NW+(3,1)` | `(13,11)` | Yes | `GAREFN RemoveOccupy1=3,1`; `NAREFN RemoveOccupy8=3,1`; east-edge `Bib=yes` opens same cell for movement | Passable/enterable with respect to refinery building | Yes |
| `NW+(4,1)` | `(14,11)` | No | `QueueingCell=4,1` comment says harvester aims here when not allowed to reserve docking cell/refinery | Outside foundation; not the pad cell | Yes |

Conclusion for this slice: the stock visible/passable pad aligns with `NW+(3,1)`. `NW+(2,1)` is not supported by stock art/remove-occupy/passability evidence.

## 6. Current Rust Implementation Status

| Surface | Current status | Evidence | Rust delta |
|---|---|---|---|
| Base vs hidden occupancy split | mostly matches current intended model | `src/sim/production/production_tech.rs:620..701` keeps `building_base_foundation_cells` separate from hidden occupancy; tests at `production_tech.rs:959..973` keep `(13,21)` despite `RemoveOccupy` | none observed for this art relation |
| Movement blocker east-edge `Bib=yes` | matches checked static stock result | `src/sim/production/production_tech.rs:718..766`; tests at `production_tech.rs:998..1028` drop the east edge and keep x-offset 2 blocked | none observed for stock `(3,1)` vs `(2,1)` |
| `NumberImpassableRows` as dynamic/contact gate | represented as non-static state path | `src/sim/production/production_tech.rs:731..766` only applies when `number_rows_active` is true | broad live `Can_Enter_Cell` parity remains outside this slot |
| Miner `refinery_pad_cell` fallback | contradicts stock art/passability relation | `src/sim/miner/miner_dock_sequence.rs:116..130` returns `rx+2, ry+1` when no docking offset exists | mismatch if this function is meant to name stock refinery visible/passable pad |
| Coord-cell docs/parity row | stale/misleading | `coord-cell-conversions/fn-building-getdockcoord.md` labels `+0x16BC` branch as refinery; `_parity.md` row 23/35 says stock GAREFN/NAREFN pad fixed to `NW+(2,1)` | docs should be corrected |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock `GAREFN` rules keys | verified | `rulesmd.ini:11722..11764` | none |
| Stock `NAREFN` rules keys | verified | `rulesmd.ini:12515..12524` | none |
| Stock `GAREFN` art keys | verified | `artmd.ini:1763..1795` | none |
| Stock `NAREFN` art keys | verified | `artmd.ini:1706..1760` | none |
| `AddOccupy`/`RemoveOccupy` parser/storage | verified | `0x0045FE50`, strings `0x0081A634`, `0x0081A624` | exact constructor default not re-decompiled in this slot; covered by prior report |
| Base foundation placement | verified | `0x00441F60` | none for checked cells |
| Hidden occupancy writer | verified | `0x005683C0` | downstream visual readers of `Cell+0x100` outside scope |
| `Bib=yes` east-edge skip | verified | `0x0073F0A0`, `BuildingType+0x1570`, east-neighbor probe | none for checked cells |
| `NumberImpassableRows=3` helper | verified | `0x00458A00` | full radio/contact lifecycle outside scope |
| `GetDockCoord +0x16BC` stock applicability | verified | `0x00447B20`, `0x0045FE50` maps `+0x16BC` to `Weeder` | slots 1-2 can deepen branch ownership, but stock-art conclusion is settled |
| Rust art/passability status | touched-not-exhausted | files listed in section 6 | no tests run in this research-only slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Which stock art cells name RemoveOccupy for GAREFN/NAREFN? -> GAREFN has `RemoveOccupy1=3,1`; NAREFN has `RemoveOccupy8=3,1`.` (evidence: `artmd.ini:1795`, `1760`)
- `[RESOLVED] OQ-2 - Does stock art name `NW+(2,1)` as RemoveOccupy or QueueingCell? -> No; QueueingCell is `4,1`.` (evidence: `artmd.ini:1716`, `1773`, `1795`, `1760`)
- `[RESOLVED] OQ-3 - Do Add/RemoveOccupy remove real building object-list footprint? -> No; base foundation placement/enter uses the foundation list, while Add/Remove modifies `Cell+0x100`.` (evidence: `0x00441F60`, `0x005683C0`)
- `[RESOLVED] OQ-4 - Which stock cell is east-edge-passable under Bib? -> x-offset 3 cells, including `NW+(3,1)`; x-offset 2 remains blocked.` (evidence: `0x0073F0A0`, `rulesmd.ini:11730`, `12523`)
- `[RESOLVED] OQ-5 - Does contacted `NumberImpassableRows=3` open x-offset 2? -> No; helper protects cells with `cell_x < origin_x + 3`, so x-offset 2 remains protected and x-offset 3 is the first allowed column.` (evidence: `0x00458A00`, `rulesmd.ini:11764`, `12524`)
- `[RESOLVED] OQ-6 - Is `+0x16BC` the stock refinery flag? -> No for stock refineries; `0x0045FE50` reads `Weeder` into `+0x16BC`, while stock refineries use `DockUnload` at `+0x16B3` and `Refinery` at `+0x16BB`.` (evidence: `0x0045FE50`, `rulesmd.ini:11726`, `11727`, `12519`, `12520`)
- `[RESOLVED] OQ-7 - Is `NW+(4,1)` the pad? -> No; it is `QueueingCell`, outside the 4x3 foundation and commented as the harvester aim cell when reservation is not allowed.` (evidence: `artmd.ini:1716`, `1773`)
- `[RESOLVED] OQ-8 - Current Rust production footprint relation? -> Current production helpers keep base foundation and hidden occupancy separate and keep the east edge open for `Bib=yes`.` (evidence: `src/sim/production/production_tech.rs:620..766`)
- `[RESOLVED] OQ-9 - Current Rust miner pad naming relation? -> `refinery_pad_cell` now returns `NW+(2,1)` without a docking offset, which conflicts with stock art/passability pad wording if this names the stock visible/passable pad.` (evidence: `src/sim/miner/miner_dock_sequence.rs:116..130`)
- `[DEFERRED] OQ-10 - Full radio accepted-cell proof for stock `CAN_DOCK(0x0E)`.` (category: out-of-scope; reason: covered by other swarm slots and existing miner docs; next-step-if-pursued: read slot 3 report)
- `[DEFERRED] OQ-11 - Runtime replay visualization of the exact presented harvester frame over `(13,11)`.` (category: needs-runtime-debugger; reason: this slot is static INI/binary research; next-step-if-pursued: capture stock GAREFN docking replay)
- `[DEFERRED] OQ-12 - Non-stock Weeder/TS path using `+0x16BC`.` (category: out-of-scope; reason: target is stock YR GAREFN/NAREFN; next-step-if-pursued: dedicated Weeder/GetDockCoord report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock refinery visible/passable pad is `NW+(3,1)`, while `NW+(2,1)` remains blocked by the refinery object | `artmd.ini:1795`, `1760`; `0x0073F0A0`; `0x00458A00` | `refinery_pad_cell` returns `NW+(2,1)` if used as stock pad | `src/sim/miner/miner_dock_sequence.rs::refinery_pad_cell`; tests in `src/sim/miner/miner_tests.rs` | Do not name or assert `NW+(2,1)` as the stock GAREFN/NAREFN pad; stock pad/passable cell should be `NW+(3,1)` unless this function is explicitly scoped to Weeder/GetDockCoord branch behavior | Test proposal: `stock_refinery_pad_cell_uses_removeoccupy_passable_east_edge` should place GAREFN at `(10,10)` and assert pad/passable cell `(13,11)` with `(12,11)` blocked | Do not apply `GetDockCoord +0x16BC` Weeder branch to stock refineries |
| `RemoveOccupy` is hidden occupancy, not the normal building movement footprint | `0x00441F60`, `0x005683C0`; prior `REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md` | current production helpers mostly match; keep this separation | `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs` | Continue keeping `building_base_foundation_cells` separate from hidden occupancy; movement passability should not be implemented by deleting `(3,1)` from `Cell+0xE4` | Test proposal: `stock_refinery_removeoccupy_does_not_delete_base_foundation_cell` should assert base foundation includes `(13,11)` while hidden occupancy removes/adjusts it | Do not conflate `Cell+0x100` hidden counters with `Cell+0xE4` object-list passability |
| `QueueingCell=4,1` is outside the foundation and is not the physical pad | `artmd.ini:1716`, `1773`; foundation `4x3` at `artmd.ini:1709`, `1766` | Rust already has separate `refinery_can_dock_queue_cell(rx,ry) -> (rx+3,ry+1)` naming confusion should be audited with other slots | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs` | Keep queue/wait cell, accepted/passable pad cell, and any Weeder GetDockCoord cell as distinct named concepts | Test proposal: `stock_refinery_queueing_cell_stays_outside_pad_and_foundation` should assert queue `(14,11)` is separate from pad `(13,11)` for GAREFN `(10,10)` | Do not use QueueingCell as proof for either `NW+(2,1)` or accepted dock anchor |

## 10. Negative Facts / Do Not Do

- Do not call `NW+(2,1)` the stock `GAREFN`/`NAREFN` art pad. Active in YR: No; the stock art pad/opened east-edge cell is `NW+(3,1)`.
- Do not use `BuildingTypeClass+0x16BC` as a stock refinery flag. Active in YR: No for stock refineries; binary maps it to `Weeder`.
- Do not use `RemoveOccupy` to delete real building foundation/object-list cells. Active in YR: No; it modifies hidden occupancy `Cell+0x100`.
- Do not merge `QueueingCell=4,1` with the physical pad. Active in YR: No; it is outside `Foundation=4x3` and named/commented as a waiting aim point.
- Do not "fix" stock miner deposit tests to `(12,11)` from this art evidence. Active in YR: No; this slot's art/passability evidence supports `(13,11)`.

## 11. Stale Docs / Follow-up Docs

- `docs/research/coord-cell-conversions/fn-building-getdockcoord.md`: replace "Only branch 1 is active for refineries in standard YR" with: "Branch 1 is gated by `BuildingTypeClass+0x16BC`, which `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` maps to `Weeder`, not stock `Refinery`. Stock `GAREFN`/`NAREFN` set `DockUnload=yes` (`+0x16B3`) and `Refinery=yes` (`+0x16BB`) and do not take the `+0x16BC` `NW+(2,1)` branch."
- `docs/research/coord-cell-conversions/fn-building-getdockcoord.md`: replace "refinery pad branch" with "`+0x16BC` Weeder/GetDockCoord branch" unless a later Weeder-specific report proves a narrower live YR name.
- `docs/research/coord-cell-conversions/_parity.md`: replace row `BuildingClass__GetDockCoord refinery branch FIXED` wording with: "Stale/incorrect for stock refineries: `+0x16BC` is Weeder, not stock `GAREFN`/`NAREFN`; stock art/passability pad is `NW+(3,1)`, while `NW+(2,1)` remains blocked. Reclassify the row as DRIFT or split into `GetDockCoord +0x16BC` and `stock refinery dock/pad` rows."

## 12. Remaining Uncertainty

- Exact full radio accepted-cell path is intentionally left to the companion slots; this report uses existing reports and local binary only to avoid expanding beyond art/passability.
- Runtime visual replay frame capture was not performed; the conclusion is from static retail INI plus decompiled active passability and occupancy paths.
- Non-stock `Weeder=yes` use of `GetDockCoord +0x16BC` remains outside this stock refinery slice.

## Sources

- Ghidra read-only: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, `BuildingClass::Place_OccupyMap @ 0x00441F60`, `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`, `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `FUN_00458A00 @ 0x00458A00`, `BuildingClass__GetDockCoord @ 0x00447B20`.
- Ghidra strings: `QueueingCell @ 0x0081A614`, `RemoveOccupy%d @ 0x0081A624`, `AddOccupy%d @ 0x0081A634`, `CanHideThings @ 0x0081A640`, `Refinery @ 0x0081AA5C`, `DockUnload @ 0x0081AA94`, `NumberImpassableRows @ 0x0081AD6C`.
- INI: `ini/rulesmd.ini`, `ini/artmd.ini`.
- Existing binary-backed docs: `docs/research/REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md`, `docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`, `docs/research/miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/rules/art_data.rs`, `src/rules/object_type.rs`.
