# Infantry Building Occupant Pathing - Ghidra Research Report

**Address(es):** `0x0051BF90` `InfantryClass::Can_Enter_Cell`, contrast-only `0x0073F0A0` `UnitClass::Can_Enter_Cell`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** building-occupied ground cells inside `InfantryClass::Can_Enter_Cell`, specifically whether infantry has vehicle-equivalent handling for HasBib, NumberImpassableRows, RadioClass contacts, UnitRepair/Bunker, gates, laser fences, and firestorm-wall style flags.  
**Non-Scope:** full infantry subcell allocation, bridge behavior except to avoid misattribution, vehicle behavior except contrast at already-settled branches, placement validators.  
**Confidence:** High for branch presence/absence in the decompiled infantry function; Medium-High for human-readable flag names inherited from prior BuildingType field audits.  
**Active in YR:** Yes for the infantry function and normal building-object branch; Conditional for individual building-type flags depending on stock/mod INI data.

## 1. Overview

`InfantryClass::Can_Enter_Cell` is not a copy of the vehicle building-occupant path. It reuses the same A* slot shape and bridge sub-check, but its building-object loop lacks the vehicle-only `NumberImpassableRows` helper callsites, RadioClass contact-vector branch, and HasBib east-neighbor relaxation.

The infantry building branch instead tests building-type flags directly: invisible/radiation-like `+0x1701`, `LaserFence +0x16BF`, `FirestormWall +0x16C0`, `Gate +0x16B7`, and then ownership/weapon/capability rules. Active in YR: Yes for the branch; Conditional for each flag by building data.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning used in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `vtable+0x1AC` | `InfantryClass` | A* `Can_Enter_Cell` entry | vtable binding from `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`; decompile `0x0051BF90` | Yes |
| `vtable+0x1B0` | `InfantryClass` | shared `CheckBridgeTraversal` sub-check | decompile `0x0051BF90` calls through `+0x1B0`; `0x004D9C60` body | Yes |
| `CellClass+0xE4/+0xE8` | cell | ground/bridge object-list heads | infantry decompile selects `+0xE4` when packed layer byte is 0, `+0xE8` otherwise | Yes |
| `BuildingType+0x16B7` | building type | `Gate=` / gate-style building flag | assembly `0x0051C4EB`; prior field docs `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md` | Conditional; stock YR gates set `Gate=yes` |
| `BuildingType+0x16BF` | building type | `LaserFence=` | assembly `0x0051C4A6`; prior field docs | Conditional; no stock YR `LaserFence=yes` found in `rulesmd.ini` |
| `BuildingType+0x16C0` | building type | `FirestormWall=` TS-era flag | assembly `0x0051C4C8`; prior field docs | Conditional / data-inert in stock YR |
| `BuildingType+0x1701` | building type | invisible/radiation/bridge-repair-hut style exclusion flag, name varies by subsystem docs | assembly `0x0051C498`; prior docs disagree on name | Conditional |
| `BuildingType+0x1620` | building type | `NumberImpassableRows` | helper `0x00458A00`; not read by `0x0051BF90` | No for infantry branch |
| `BuildingType+0x1570` | building type | `Bib=yes` / HasBib | vehicle branch assembly `0x0073F7D3`; not read by `0x0051BF90` | No for infantry branch |
| `BuildingType+0x16A9/+0x16AB` | building type | `UnitRepair=yes` / `Bunker=yes` row-helper gates | vehicle branch assembly `0x0073F74B..0x0073F75D`; not read by `0x0051BF90` | No for infantry branch |

## 3. Core Logic

### A. Shared entry shape, not shared building policy

`0x0051BF90` starts with the same packed layer pre-decision, tube checks, and `vtable+0x1B0` bridge sub-check as `UnitClass::Can_Enter_Cell`. Active in YR: Yes. Evidence: decompile `0x0051BF90`; bridge hierarchy report vtable slot `0x007EB204 -> 0x0051BF90`, `0x007EB208 -> 0x004D9C60`.

After the bridge sub-check, infantry enters its own object-list loop over `cell+0xE4` or `cell+0xE8`. Active in YR: Yes. Evidence: decompile `0x0051BF90` selects `*(iStack_10+0xE4)` when layer byte is zero, else `*(iStack_10+0xE8)`.

### B. Negative vehicle-equivalence findings

The vehicle `NumberImpassableRows` helper `0x00458A00` is not called from `0x0051BF90`. The infantry decompile contains no call to `0x00458A00`, no reads of `BuildingType+0x1620`, and no reads of `BuildingType+0x16A9/+0x16AB` in its building branch. Active in YR: No for infantry. Evidence: decompile `0x0051BF90`; helper body `0x00458A00`; contrast assembly `0x0073F74B..0x0073F76D` in `UnitClass::Can_Enter_Cell`.

The vehicle RadioClass contact-vector path is not present in infantry. `UnitClass::Can_Enter_Cell` calls `DynamicVectorClass::Contains @ 0x0065AD50` at `0x0073F58A` before the row helper; `0x0051BF90` has no equivalent `Contains` call. Active in YR: No for infantry. Evidence: decompile `0x0051BF90`; `DynamicVectorClass::Contains @ 0x0065AD50`; prior contact-vector report.

The vehicle HasBib path is not present in infantry. `UnitClass::Can_Enter_Cell` reads `BuildingType+0x1570` at `0x0073F7D3` and probes `(x+1,y)` via `0x0089F690`; `0x0051BF90` has no `+0x1570` read and no adjacent-cell building probe for bib relaxation. Active in YR: No for infantry. Evidence: decompile `0x0051BF90`; assembly context `0x0073F7D3..0x0073F80F`.

### C. Infantry-specific building handling

For a building occupant (`WhatAmI()==6`), infantry first checks `BuildingType+0x1701` and `+0x16BF`. If `+0x1701` is set, or if `+0x16BF` is set and building state `+0x618` is not `0xC` or `8`, the branch leaves the special building handling path and goes to the generic object logic. Active in YR: Conditional. Evidence: assembly `0x0051C498..0x0051C4C2`.

If `BuildingType+0x16C0` is set, infantry reads the owner/house byte at owner `+0x1FA`; when nonzero it returns code `7` hard-block, otherwise it falls through to generic handling. Active in YR: Conditional / stock-data-inert for standard YR firestorm walls. Evidence: assembly `0x0051C4C8..0x0051C4E6`; prior field docs say `+0x16C0 = FirestormWall`.

If `BuildingType+0x16B7` is set, infantry calls `BuildingClass__CanGarrison @ 0x004525F0`. If `CanGarrison` returns true, the building does not upgrade the block code in that branch. If false, allied building upgrades the result to at least code `3`; enemy building requires infantry action ability (`vtable+0x2AC`) or returns `7`, otherwise upgrades to at least code `5`. Active in YR: Conditional; stock gates and many civilian buildings set related flags, while actual acceptance depends on building mission/state and ownership. Evidence: assembly `0x0051C4EB..0x0051C549`; decompile `0x004525F0`.

Generic enemy building/object handling later requires infantry weapon range for hostile occupied cells. If the final cell-owner path reaches the enemy-owner check and `TechnoClass__GetWeaponRange(this,-1) < 1`, infantry returns `7`; otherwise it can return/upgrade to code `5`. Active in YR: Yes. Evidence: decompile `0x0051BF90` final owner check; prior bridge hierarchy report §3.6.

## 4. INI Keys

| Key | Stock YR evidence | Infantry effect in this slice | Active in YR |
|---|---|---|---|
| `Bib=yes` | `rulesmd.ini` examples: `GAREFN`, `NAREFN`, war factories | No direct infantry building-occupant relaxation; vehicle-only HasBib branch | No for infantry |
| `NumberImpassableRows=` | stock refineries/factories/depots/NATBNK/CAOUTP | No direct infantry row-helper call | No for infantry |
| `UnitRepair=yes` | stock repair depots/outposts/naval yards | No direct infantry `+0x16A9` row-helper branch in `0x0051BF90` | No for infantry branch |
| `Bunker=yes` | stock `NATBNK` | No direct infantry `+0x16AB` row-helper branch in `0x0051BF90` | No for infantry branch |
| `Gate=yes` | stock `rulesmd.ini:17204` | Infantry reads `BuildingType+0x16B7` and calls `CanGarrison` path | Conditional |
| `LaserFence=` | no active stock YR key found | Infantry reads `+0x16BF`; stock data appears inert | Conditional / No in stock YR data |
| `FirestormWall=` | TS-era field, no stock YR activation found | Infantry reads `+0x16C0`; owner flag can hard-block | Conditional / No in stock YR data |

## 5. Integration Points

`AStar_main_loop @ 0x00429A90` dispatches through per-class `vtable+0x1AC`; Infantry binds that slot to `0x0051BF90`. Active in YR: Yes. Evidence: `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`, vtable read at `0x007EB204`.

Runtime infantry movement calls the same five-argument shape through walk locomotion. Active in YR: Yes. Evidence: `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`, walk call around `0x0075B669..0x0075B690`.

`Look_up_building_in_cell @ 0x0047C520` remains a ground-list helper, but in this slice it is evidence for the vehicle HasBib/row-helper contrast, not an infantry bib/row path. Active in YR: Yes as helper; not used for infantry bib/row equivalence. Evidence: decompile `0x0047C520`.

## 6. Current Rust Implementation Status

Current Rust surfaces scanned: `src/sim/pathfinding/cell_entry.rs` and `src/sim/movement/movement_occupancy.rs`. `cell_entry.rs` has shared result codes and generic blocker classification; `movement_occupancy.rs` distinguishes `DeferredCellCheck::Infantry` vs `Vehicle`, but both eventually call the same `classify_occupied_cell_with_layers`.

Rust currently has no verified infantry-specific suppression of vehicle building exceptions in `classify_occupied_cell_with_layers`; future work should ensure any HasBib/NumberImpassableRows/RadioContact building skip is vehicle-only unless a separate binary investigation proves another infantry path. Current Rust delta: unchecked for any in-progress vehicle exception patch; no Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `InfantryClass::Can_Enter_Cell @ 0x0051BF90` building-object branch | verified | decompile `0x0051BF90`; assembly contexts `0x0051C498..0x0051C549` | none for scoped flags |
| Absence of `NumberImpassableRows` helper from infantry | verified | decompile `0x0051BF90`; helper `0x00458A00`; vehicle contrast `0x0073F76D` | none for direct helper call |
| Absence of RadioClass contact-vector row branch from infantry | verified | decompile `0x0051BF90`; `DynamicVectorClass::Contains @ 0x0065AD50`; vehicle contrast `0x0073F58A` | none for direct equivalent |
| Absence of HasBib branch from infantry | verified | decompile `0x0051BF90`; vehicle contrast assembly `0x0073F7D3..0x0073F80F` | none for direct branch |
| Gate/Laser/Firestorm flags in infantry | verified | assembly contexts `0x0051C498..0x0051C4F7`; field docs | exact player-visible gate-open scenarios not runtime-traced |
| Full infantry subcell occupancy | deferred | user non-scope | separate infantry subcell investigation |
| Full bridge behavior | deferred | user non-scope; prior bridge docs exist | separate bridge trace if needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `0x0051BF90` the Infantry A* `Can_Enter_Cell` entry? -> Yes, by vtable+0x1AC binding and runtime walk call docs.` Evidence: `0x007EB204`, `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`.
- `[RESOLVED] OQ-2 - Does infantry call the NumberImpassableRows helper `0x00458A00`? -> No direct call or `+0x1620` read appears in the full decompile.` Evidence: `0x0051BF90`, `0x00458A00`.
- `[RESOLVED] OQ-3 - Does infantry have the vehicle RadioClass contact-vector row branch? -> No equivalent `DynamicVectorClass::Contains @ 0x0065AD50` call appears in `0x0051BF90`.` Evidence: `0x0051BF90`, vehicle contrast `0x0073F58A`.
- `[RESOLVED] OQ-4 - Does infantry have the vehicle HasBib branch? -> No `BuildingType+0x1570` read or east-neighbor probe appears in `0x0051BF90`.` Evidence: `0x0051BF90`, vehicle contrast `0x0073F7D3..0x0073F80F`.
- `[RESOLVED] OQ-5 - Does infantry read gate/laser/firestorm-style building flags? -> Yes: `+0x16B7`, `+0x16BF`, `+0x16C0`, and `+0x1701` are read in the building branch.` Evidence: assembly `0x0051C498..0x0051C4F7`.
- `[RESOLVED] OQ-6 - Is the gate path unconditional passability? -> No; `+0x16B7` calls `BuildingClass__CanGarrison`, then ownership/action gates choose codes `3`, `5`, or `7`.` Evidence: `0x0051C4EB..0x0051C549`, `0x004525F0`.
- `[RESOLVED] OQ-7 - Are stock YR LaserFence/Firestorm flags active by data? -> No stock `LaserFence=` or `FirestormWall=` activation found in `rulesmd.ini`; code remains live for mods/TS-style data.` Evidence: INI grep; field docs.
- `[DEFERRED] OQ-8 - What exact visual/player scenario proves gate-open infantry passability for every stock gate mission state?` Category: requires-different-system-context; reason: this slice verified branch shape, not gate mission runtime transitions; next-step-if-pursued: trace a stock gate open/closed `InfantryClass::Can_Enter_Cell` call.
- `[DEFERRED] OQ-9 - Does any non-direct helper inside infantry reproduce row-helper semantics indirectly?` Category: bounded-cost-too-high; reason: the scoped vehicle-equivalent direct branches are absent, but exhaustive semantic clone search across all callees was outside this slot; next-step-if-pursued: binary pattern search for `+0x1620` and candidate-cell X compare outside `0x00458A00`.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Infantry must not receive vehicle HasBib east-edge building relaxation. | `0x0051BF90` has no `+0x1570` read; vehicle branch at `0x0073F7D3..0x0073F80F`. | unchecked / likely shared classifier risk | `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs` | Gate any future HasBib building skip to vehicle/unit movement, not `EntityCategory::Infantry`. | `infantry_pathing_does_not_use_hasbib_east_edge_relaxation`: infantry path to a `Bib=yes` refinery east-edge occupied cell remains blocked/classified by infantry building rules while a vehicle-specific test may relax it. | Do not put HasBib into shared `find_primary_blocker` without mover-class gating. |
| Infantry must not use `NumberImpassableRows` / RadioClass contact-vector building skip. | `0x0051BF90` has no `0x00458A00` or `0x0065AD50` call; vehicle branch has `0x0073F58A` and `0x0073F76D`. | unchecked / likely shared classifier risk | `src/sim/pathfinding/cell_entry.rs`; any future radio/contact movement surface | Vehicle row/contact exceptions should require vehicle/unit context plus contact state; infantry contacting or targeting a building should not skip building cells by row count. | `infantry_pathing_ignores_number_impassable_rows_contact_skip`: infantry attempting to enter a depot/refinery foundation cell does not become clear merely because the building has `NumberImpassableRows` or a contact/reservation record. | Do not model `NumberImpassableRows` as a universal static erasure for all mover categories. |
| Infantry does have a separate gate/garrison-style building path using `+0x16B7` and `BuildingClass__CanGarrison`. | assembly `0x0051C4EB..0x0051C549`; decompile `0x004525F0`. | missing/unchecked | `src/sim/pathfinding/cell_entry.rs`; possibly building state/rules surfaces | Preserve infantry-specific building result codes: CanGarrison true can allow continuing without hard-blocking; CanGarrison false yields allied code `3`, enemy code `5` if infantry can act, or `7` if it cannot. | `infantry_gate_building_uses_can_garrison_result_codes`: gate/garrison fixture asserts allied false-garrison returns code 3, enemy no-action infantry returns code 7, enemy armed infantry returns code 5. | Do not replace this with vehicle UnitRepair/Bunker or HasBib logic; gate flag `+0x16B7` is a separate branch. |

### Stale Docs / Follow-up Docs

- `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` contains stale offset names around its building branch. Replacement wording: "`BuildingType+0x16B7` is the gate/garrison-style flag used by `InfantryClass::Can_Enter_Cell` and drawing docs; `BuildingType+0x16BF` is `LaserFence=`, and `BuildingType+0x16C0` is `FirestormWall=` / TS-era data-inert in stock YR. Do not describe `+0x16BF` as `IsGate` or `+0x16C0` as `IsLaserFence` without reconciling against current field docs."

## Negative Facts / Do Not Do

- Do not apply HasBib east-edge relaxation to infantry. Evidence: no `+0x1570` read in `0x0051BF90`; vehicle-only evidence at `0x0073F7D3..0x0073F80F`. Active in YR: No for infantry.
- Do not apply `NumberImpassableRows` helper `0x00458A00` to infantry building occupants. Evidence: no helper call in `0x0051BF90`; helper is vehicle-branch evidence at `0x0073F5A2`/`0x0073F76D`. Active in YR: No for infantry.
- Do not treat RadioClass contact-vector row skip as infantry behavior. Evidence: no `DynamicVectorClass::Contains @ 0x0065AD50` call in `0x0051BF90`; vehicle call at `0x0073F58A`. Active in YR: No for infantry.
- Do not conflate `Bunker=yes` / `UnitRepair=yes` row-helper branches with infantry gate/garrison handling. Evidence: infantry reads `+0x16B7`, not `+0x16A9/+0x16AB`, in the scoped building branch. Active in YR: No for infantry row-helper branches.
- Do not assume stock YR laser-fence/firestorm pathing cases occur in normal skirmish data. Evidence: code reads `+0x16BF/+0x16C0`, but stock `rulesmd.ini` grep found no active `LaserFence=`/`FirestormWall=` keys. Active in YR: Conditional/code-live, data-inert in stock.

## Remaining Uncertainty

- Exact runtime gate mission/state scenarios for `+0x16B7` need a concrete gate-open/closed trace if implementation wants more than branch-level parity.
- A full semantic clone search for indirect row-helper equivalents outside direct `0x00458A00` calls was not performed; direct vehicle-equivalent branches are absent in the infantry function.
- `BuildingType+0x1701` naming varies across prior docs; this report uses it only as a scoped branch flag, not a canonical semantic name.

## Sources

- Ghidra decompile: `0x0051BF90`, `0x0073F0A0`, `0x004D9C60`, `0x004525F0`, `0x0047C520`, `0x00458A00`, `0x0065AD50`.
- Ghidra assembly context: `0x0051C498..0x0051C549`, `0x0073F57C..0x0073F58F`, `0x0073F74B..0x0073F766`, `0x0073F7D3..0x0073F80F`.
- Existing docs: `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`, `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`, `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md`, `BIB_ADJACENT_CELL_DIRECTION_SOURCE_GHIDRA_REPORT.md`, `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`, `BUILDINGTYPECLASS_FIELDS.csv`.
- INI checks: `ini/rulesmd.ini`, `ini/rules.ini`.
