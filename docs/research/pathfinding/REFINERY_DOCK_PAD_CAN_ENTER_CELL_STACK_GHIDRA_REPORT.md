# Refinery Dock Pad Can Enter Cell Stack - Ghidra Research Report

**Address(es):** `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `FUN_00458A00 @ 0x00458A00`, `BuildingClass::Place_OccupyMap @ 0x00441F60`, `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`, `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`, `TechnoClass__Set_Destination @ 0x00741970`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR `GAREFN`/`NAREFN` refinery `UnitClass::Can_Enter_Cell` stack for relative cells `(3,1)`, `(2,1)`, `(3,0)`, `(3,2)`, queue cell `(4,1)`, and the contacted-harvester exception.  
**Non-Scope:** full refinery unload FSM, full A* caller tree, non-refinery buildings, aircraft/carryall docking, and visual `BEHIND` marker details.  
**Confidence:** High for the checked cells and binary decision stack; Medium for Rust current-status comments where future concurrent changes may alter path-grid wiring.  
**Active in YR:** Yes for stock `GAREFN`/`NAREFN`; conditional parts are gated by `Bib=yes`, `NumberImpassableRows=3`, `RadioClass` contact membership, and `CanHideThings`.

## 1. Overview

For stock Allied/Soviet refineries, `AddOccupy` and `RemoveOccupy` do not define the real building object-list footprint seen by `UnitClass::Can_Enter_Cell`. The real passability stack sees the base `Foundation=4x3` building object on all twelve foundation cells, then conditionally skips that building through either the contacted-building `NumberImpassableRows` branch or the `HasBib` east-edge branch.

The checked cell result for stock `GAREFN`/`NAREFN` is: `(3,0)`, `(3,1)`, and `(3,2)` are not blocked by the refinery building; `(2,1)` remains blocked; `(4,1)` is outside the foundation and has no refinery object-list blocker. These outcomes are before considering other occupants such as a waiting harvester.

## 2. Key Offsets / Data

| Item | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `BuildingTypeClass+0x1570` | `Bib=yes` / `HasBib` | parser in `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; consumer `0x0073F7D3` | Yes for `GAREFN`/`NAREFN` (`rulesmd.ini:11730`, `12523`) |
| `BuildingTypeClass+0x1620` | `NumberImpassableRows` | parser `0x0046013A`; helper `0x00458A00` | Yes; stock value `3` (`rulesmd.ini:11764`, `12524`) |
| `BuildingTypeClass+0x1624..0x1660` | `AddOccupy1..8` pairs | parser loop `0x00461425..0x00461486`; string `0x0081A634` | Conditional on hidden occupancy writers |
| `BuildingTypeClass+0x1664..0x16A0` | `RemoveOccupy1..8` pairs | parser loop `0x0046148A..0x004614E8`; string `0x0081A624` | Conditional on hidden occupancy writers |
| `BuildingTypeClass+0x1766` | `CanHideThings` gate | parser near `0x0046140F`; enter/exit writers | Yes; stock `GAREFN/NAREFN` set/retain true (`artmd.ini:1790`, `1752`) |
| `CellClass+0xE4` | ground object list scanned by building lookup and `Can_Enter_Cell` | `Look_up_building_in_cell @ 0x0047C520`; `UnitClass::Can_Enter_Cell @ 0x0073F0A0` | Yes |
| `CellClass+0x100` | hidden occupancy counter affected by `CanHideThings`/height/add/remove | writer evidence in `0x005683C0` and `0x005687F0`; reader report `CELLCLASS_0X100...` | Conditional; not a passability input |
| `RadioClass+0xE4/+0xE8` | mover contact vector used by contacted-building branch | `DynamicVectorClass__Contains @ 0x0065AD50`; prior radio vector report | Yes for dock/contact protocols |

## 3. Core Logic

### 3.1 Base foundation is the real building object list

`BuildingClass::Place_OccupyMap @ 0x00441F60` walks the foundation cell-list returned by the building foundation vtable and marks those base cells. `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` likewise adds the building object to the base multi-cell contents before hidden-occupancy work.

**Active in YR:** Yes. This is the standard building placement / cell-enter path. The base foundation for stock `GAREFN/NAREFN` is `4x3` from `artmd.ini:1766` and `1709`.

**Finding:** `AddOccupy`/`RemoveOccupy` are not read by `Place_OccupyMap` and do not remove `(3,1)` from the real `CellClass+0xE4` building list. The dock pad is still a cell containing the refinery building object; it becomes enterable through later `Can_Enter_Cell` skip logic.

### 3.2 AddOccupy/RemoveOccupy affect hidden occupancy only

The add/remove pairs are parsed from the art/image section into fixed eight-slot arrays. The hidden occupancy writers apply them only under `CanHideThings`: enter increments height/add cells and decrements listed remove cells if nonzero; exit reverses height/add effects with nonzero guards. The downstream `CellClass+0x100` readers drive hiding/behind-object visuals, not the central vehicle passability stack.

**Active in YR:** Conditional. Stock `GAREFN` has `AddOccupy1=-1,0`, `AddOccupy2=-1,-1`, `RemoveOccupy1=3,1`; stock `NAREFN` has eight `RemoveOccupy` entries including `RemoveOccupy8=3,1`. Effects require `CanHideThings`, which is true in stock art.

### 3.3 Contacted-building exception

In `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, while scanning the candidate cell's object list, the focused branch checks whether the current occupant object is in the moving unit's `RadioClass` contact vector. If the occupant is a building and `FUN_00458A00` returns false, the building object is skipped for that candidate cell.

The helper returns true for cells west of `building_origin_x + NumberImpassableRows`, and false at/after that limit. With stock `NumberImpassableRows=3`, a contacted harvester can skip the refinery building only at foundation X offset `3` and beyond. It cannot use this contact exception for `(2,1)`.

**Active in YR:** Conditional but live. `TechnoClass__Set_Destination @ 0x00741970` sends radio `0x0E` to building destinations, checks `BuildingTypeClass+0x16B3 DockUnload`, and stock `GAREFN/NAREFN` set `DockUnload=yes`. The contact vector itself is populated/cleared by live HELLO/BREAK radio paths per `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`.

### 3.4 HasBib east-edge relaxation

Later in the same object-list branch, non-laser-fence buildings with `HasBib` probe the cell one step east, using the initialized direction table value `(dx=+1, dy=0)` at `0x0089F690`. If the east neighbor does not contain the same building, the current building is skipped as a blocker for the candidate cell.

For a `4x3` base foundation, `(3,0)`, `(3,1)`, and `(3,2)` all have east neighbors outside the building, so `HasBib` skips the refinery there. `(2,1)` has east neighbor `(3,1)` containing the same refinery, so it remains blocked.

**Active in YR:** Conditional but live for stock `GAREFN/NAREFN`, which set `Bib=yes`. The direction table initializer is verified by `BIB_ADJACENT_CELL_DIRECTION_SOURCE_GHIDRA_REPORT.md`; the consumer branch is `0x0073F7D3..0x0073F80F`.

## 4. Checked Cell Matrix

Assume a stock refinery at origin `(rx, ry)`, no other blocking objects, and ordinary ground vehicle passability.

| Relative cell | Real refinery object in `Cell+0xE4`? | Contacted `NumberImpassableRows=3` result | `HasBib` result | Net building blocker result | Active in YR |
|---|---|---|---|---|---|
| `(3,1)` dock pad | Yes; base foundation includes it | helper false, so contacted building can be skipped | east neighbor `(4,1)` is not same building, so skipped | Enterable with respect to refinery building | Yes |
| `(2,1)` interior/west band | Yes | helper true, no contact skip | east neighbor `(3,1)` is same building, no bib skip | Blocked by refinery building | Yes |
| `(3,0)` east edge | Yes | helper false if contacted | east neighbor `(4,0)` is not same building | Enterable with respect to refinery building | Yes |
| `(3,2)` east edge | Yes | helper false if contacted | east neighbor `(4,2)` is not same building | Enterable with respect to refinery building | Yes |
| `(4,1)` queue cell | No, outside 4x3 foundation | no building object to skip | no building object to skip | Not blocked by refinery building; may be blocked by units | Yes |

## 5. INI Keys

| INI path | Value | Effect | Active in YR |
|---|---|---|---|
| `ini/rulesmd.ini [GAREFN] DockUnload=yes` | yes | standard refinery radio docking branch | Yes |
| `ini/rulesmd.ini [NAREFN] DockUnload=yes` | yes | standard refinery radio docking branch | Yes |
| `ini/rulesmd.ini [GAREFN]/[NAREFN] NumberOfDocks=1` | 1 | contact capacity / one dock | Yes |
| `ini/rulesmd.ini [GAREFN]/[NAREFN] Bib=yes` | yes | enables `HasBib` east-edge skip | Yes |
| `ini/rulesmd.ini [GAREFN]/[NAREFN] NumberImpassableRows=3` | 3 | contacted-building west-band protection | Conditional on contact branch |
| `ini/artmd.ini [GAREFN] Foundation=4x3` | 4x3 | base foundation object list | Yes |
| `ini/artmd.ini [NAREFN] Foundation=4x3` | 4x3 | base foundation object list | Yes |
| `ini/artmd.ini [GAREFN] QueueingCell=4,1` / `[NAREFN] QueueingCell=4,1` | `(4,1)` | external waiting cell; not a foundation cell | Yes |
| `ini/artmd.ini [GAREFN] AddOccupy1/2`, `RemoveOccupy1=3,1` | hidden occupancy modifiers | affects `Cell+0x100`, not real foundation list | Conditional |
| `ini/artmd.ini [NAREFN] RemoveOccupy1..8`, including `RemoveOccupy8=3,1` | hidden occupancy modifiers | affects `Cell+0x100`, not real foundation list | Conditional |

## 6. Current Rust Implementation Status

Current Rust already separates base foundation cells from hidden occupancy in `src/sim/production/production_tech.rs`, and `PathGrid::block_building_movement_cells` ignores add/remove modifiers in `src/sim/pathfinding/core.rs`. That matches the binary's "do not use AddOccupy/RemoveOccupy for real building movement occupancy" rule.

Risk remains around representation: Rust bakes movement blockers into `PathGrid`, while gamemd applies `NumberImpassableRows` and `HasBib` during live `UnitClass::Can_Enter_Cell` object-list evaluation. For the exact stock `GAREFN/NAREFN` cells checked here, the resulting blocked/unblocked set matches if Rust blocks only base foundation cells after `HasBib`/east-edge and does not treat add/remove modifiers as real movement cells. For broader buildings or contact-sensitive states, a future live `cell_entry`/object-list layer should keep the dynamic gates explicit.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock `GAREFN/NAREFN` INI keys | verified | `rulesmd.ini` and `artmd.ini` lines above | none |
| Base foundation object-list path | verified | `0x00441F60`, `0x005683C0` | none for checked cells |
| Add/remove parser/storage | verified | `0x00461425..0x004614E8` | none |
| Hidden occupancy writer classification | verified | `0x005683C0`, `0x005687F0`, `Cell+0x100` reader report | none for passability classification |
| Contacted-building `NumberImpassableRows` branch | verified | `0x0073F57C..0x0073F5A9`, `0x00458A00` | full radio FSM non-scope |
| `HasBib` east-edge branch | verified | `0x0073F7D3..0x0073F80F`, `0x0089F690=(+1,0)` | none |
| Standard refinery contact liveness | verified enough for scope | `TechnoClass__Set_Destination @ 0x00741970`; prior radio vector report | no full unload FSM replay |
| Rust current status | touched-not-exhausted | `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/app_init.rs`, `src/app_sim_tick.rs` | run targeted tests after any implementation change |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is (3,1) removed from the real foundation object list by RemoveOccupy? -> No; it remains a base 4x3 foundation cell, and RemoveOccupy only adjusts hidden occupancy counters.` (evidence: `0x00441F60`, `0x005683C0`, `0x0046148A..0x004614E8`)
- `[RESOLVED] OQ-2 - Which checked cells are skipped by stock HasBib? -> (3,0), (3,1), (3,2); (2,1) is not skipped.` (evidence: `0x0073F7D3..0x0073F80F`, `0x0089F690`)
- `[RESOLVED] OQ-3 - Which checked cells are skipped by contacted NumberImpassableRows=3? -> X offset 3 cells are skipped for a contacted building; X offset 2 is still protected.` (evidence: `0x00458A00`, `rulesmd.ini:11764`, `12524`)
- `[RESOLVED] OQ-4 - Is queue cell (4,1) a refinery foundation blocker? -> No; it is outside Foundation=4x3 and no refinery building object is present there.` (evidence: `artmd.ini:QueueingCell=4,1`, base foundation path)
- `[RESOLVED] OQ-5 - Is standard refinery contact live in stock YR? -> Yes, through building radio 0x0E and DockUnload=yes, with contact vector behavior covered by the prior radio report.` (evidence: `0x00741970`, `rulesmd.ini:11726`, `12519`)
- `[DEFERRED] OQ-6 - Exact return code after (2,1) remains blocked for every ownership/mission combination.` (category: out-of-scope; reason: this slot only needs blocker legality, not the whole return-code tree; next-step-if-pursued: targeted `UnitClass::Can_Enter_Cell` return-code report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock `GAREFN/NAREFN` movement blockers use base `4x3` foundation plus live skips; add/remove are hidden only | `0x00441F60`, `0x005683C0`, `0x005687F0`, `artmd.ini` refinery entries | none observed in current helpers; keep guarded | `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs` | Keep movement blocking independent from `AddOccupy/RemoveOccupy`; `(3,1)` passability must come from `HasBib`/contact skip, not from removing the real foundation cell | Build stock GAREFN at `(10,10)`; assert `(13,11)` is not movement-blocked, `(12,11)` is blocked, `(14,11)` not refinery-blocked, and hidden occupancy still computes add/remove cells separately | Do not collapse hidden occupancy and real foundation into one footprint |
| Contacted refinery exception opens only X offset `>=3` for `NumberImpassableRows=3`; X offset `2` stays blocked | `0x0073F57C..0x0073F5A9`, `0x00458A00`, `rulesmd.ini:11764/12524` | dynamic contact gate unchecked; static result matches checked stock cells because `HasBib` opens the same east column | future `cell_entry` / live building-object passability layer | Model `NumberImpassableRows` as a contact/UnitRepair/Bunker-gated object skip, not a general footprint erasure; for stock refineries ensure `(2,1)` never becomes open just because a harvester is contacted | Contacted HARV/CMIN approaching stock refinery may enter `(3,1)` but not `(2,1)` via the refinery object; queue `(4,1)` remains external | Static row filtering can be wrong for non-stock or non-bib cases |
| `HasBib` uses initialized east offset `(dx=+1,dy=0)` and skips cells whose east neighbor lacks the same building | `0x0073F7D3..0x0073F80F`; `BIB_ADJACENT_CELL_DIRECTION_SOURCE...` | implemented as east-edge filter in current movement helper | `src/sim/production::building_movement_blocking_cells`, `PathGrid::block_building_movement_cells` | For stock `4x3 Bib=yes`, leave `(3,0)`, `(3,1)`, `(3,2)` unblocked while retaining `(2,1)` | Non-contact unit pathing near a stock refinery east edge can cross `(3,0)/(3,1)/(3,2)` but not `(2,1)` | Do not resurrect older `DAT_0089F690` uncertainty or use south/bib-row semantics |

### Negative Facts / Do Not Do

- Do not use `AddOccupy`/`RemoveOccupy` as the normal building movement footprint. Evidence: `BuildingClass::Place_OccupyMap @ 0x00441F60` and `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` use base foundation lists; add/remove affect `Cell+0x100`.
- Do not make `RemoveOccupy1=3,1` physically remove the dock pad building object from `CellClass+0xE4`. Evidence: `RemoveOccupy` parser `0x0046148A..0x004614E8` feeds hidden occupancy writer logic, not `Place_OccupyMap`.
- Do not apply `NumberImpassableRows` as an unconditional placement-time prepass. Evidence: helper `0x00458A00` is reached from conditional `UnitClass::Can_Enter_Cell` callsites such as `0x0073F5A2`.
- Do not treat `QueueingCell=4,1` as the dock pad. Evidence: art says queue cell; checked binary stack shows `(4,1)` is outside foundation, while physical pad `(3,1)` is inside base foundation and opened by skip logic.
- Do not rely on the stale `DAT_0089F690` unknown wording. Evidence: `BIB_ADJACENT_CELL_DIRECTION_SOURCE_GHIDRA_REPORT.md` verifies startup initialization to east `(1,0)`.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BIB_SYSTEM_GHIDRA_REPORT.md`: replace the `DAT_0089F690` uncertainty wording with: "`0x0089F690` is initialized by `Foundation_direction_table_init @ 0x0049F2F0` through the CRT constructor table to signed cell offset `(dx=+1, dy=0)`, so the `HasBib` branch relaxes the east edge of the actual building object-list footprint."
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`: replace "`BuildingTypeClass+0x1664` stores AddOccupy and further on RemoveOccupy" with: "`BuildingTypeClass+0x1624..0x1660` stores `AddOccupy1..8`; `BuildingTypeClass+0x1664..0x16A0` stores `RemoveOccupy1..8`; both are hidden-occupancy modifiers, not normal foundation occupancy."

### Remaining Uncertainty

- Exact `UnitClass::Can_Enter_Cell` return code after `(2,1)` remains blocked can vary by ownership/mission/crush state and is intentionally outside this blocker-legality slice.
- Runtime tests were not run; this report is static Ghidra/INI/Rust scan evidence.

## Sources

- Ghidra read-only decompiled: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `FUN_00458A00 @ 0x00458A00`, `Look_up_building_in_cell @ 0x0047C520`, `BuildingClass::Place_OccupyMap @ 0x00441F60`, `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, `TechnoClass__Set_Destination @ 0x00741970`.
- Ghidra assembly context: `0x00461425..0x004614E8`, `0x0073F57C..0x0073F5A9`, `0x0073F7D3..0x0073F80F`.
- Prior reports: `BIB_ADJACENT_CELL_DIRECTION_SOURCE_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`, `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`, `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`.
- Rust scan: `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`.
