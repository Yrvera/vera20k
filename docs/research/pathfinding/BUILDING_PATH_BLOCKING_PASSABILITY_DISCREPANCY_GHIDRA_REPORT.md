# Building Path Blocking Passability Discrepancy - Ghidra Report

**Address(es):** `0x0073F0A0`, `0x005683C0`, `0x005687F0`, `0x00441F60`, `0x00458A00`, `0x0049F300`
**Investigation Mode:** coverage-map
**Claimed Scope:** Enough unit-vs-building cell entry behavior to size whether current Rust building blocker generation is too strict, too loose, or special-case wrong for common stock structures.
**Non-Scope:** Full exhaustive `UnitClass::Can_Enter_Cell`, runtime debugger confirmation of `DAT_0089F690`, full `NumberImpassableRows` caller matrix, and any implementation.
**Confidence:** High for base foundation/add-remove split and the UnitClass HasBib branch; Medium for final row-count severity because full caller/branch context was not exhausted.
**Active in YR:** Yes for standard building object lists and unit cell entry; Conditional for `CanHideThings`, HasBib, and `NumberImpassableRows` keyed effects.

## 1. Overview

The current Rust blocker grid is not just "a little off"; it conflates three gamemd concepts that are separate in the binary:

1. Base foundation content lists: normal building objects are added to every base foundation cell.
2. HasBib/NumberImpassableRows passability relaxation: `UnitClass::Can_Enter_Cell` may skip some building cells at entry time.
3. Hidden occupancy counters: `CanHideThings`/`OccupyHeight`/`AddOccupy`/`RemoveOccupy` update `CellClass+0x100`, not the base building object list.

Net sizing: Rust is **too strict** where it blocks `AddOccupy` visual/hidden cells as movement blockers, **too loose** where it removes `RemoveOccupy` cells from the base foundation before binary passability has a chance to classify them, and **special-case wrong** for structures whose passability depends on HasBib/NumberImpassableRows rather than a precomputed adjusted footprint.

## 2. Load-Bearing Binary Findings

| Finding | Active in YR | Evidence | Player-visible severity |
|---|---|---|---|
| `BuildingClass::Place_OccupyMap` walks only the vtable `+0x108` base foundation list, terminated by `(0x7FFF,0x7FFF)`. It does not read `AddOccupy`/`RemoveOccupy` and does not add a bib row. | Yes. This is the standard placed-building occupancy path. | `0x00441F60` decompile; prior `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`. | Severe/Frequent. Any precomputed grid that mutates the base footprint changes which cells units consider building-occupied. |
| `TechnoClass__EnterCell_AddToMultiCells` first adds the object to every base foundation cell, then separately updates `CellClass+0x100` hidden counters if the object is a building and `CanHideThings` is true. | Conditional. Standard buildings enter this path; hidden counter work requires type `+0x1766 CanHideThings != 0`, default true and many stock structures set true. | `0x005683C0`; fields `+0x1766`, `+0xEF8`, `+0x1624..0x1660`, `+0x1664..0x16A0`. | Moderate. Hidden counters are player-visible mainly through behind-building hiding/occlusion, not direct path blocking in the checked UnitClass path. |
| `RemoveOccupy` does not remove the building object from base content lists. On enter it decrements `CellClass+0x100` only if nonzero; on exit it is not symmetrically re-added because it only canceled an enter-side hidden increment. | Conditional. Same gate: building object and `CanHideThings`. | `0x005683C0` remove loop; `0x005687F0` exit only reverses diagonal and AddOccupy increments. | Severe/Frequent when Rust uses `RemoveOccupy` to make movement cells passable. Common factories/refineries use internal RemoveOccupy entries. |
| `UnitClass::Can_Enter_Cell` selects the ground or bridge object list from `cell+0xE4`/`cell+0xE8` and iterates building occupants. It does not directly consult `CellClass+0x100` in the decompiled path. | Yes. This is the central unit cell-entry predicate. | `0x0073F0A0`: `piVar15 = *(cell+0xE4/E8)`, then building branch uses object type checks. | Severe/Frequent. Building movement blocking is object-list based first, not hidden-counter based. |
| HasBib (`BuildingType+0x1570`) is a live branch in `UnitClass::Can_Enter_Cell`: it probes `cell + DAT_0089F690`; if that adjacent cell does not contain the same building, it skips this building occupant for the current cell. | Conditional. Stock refineries/war factories set `Bib=yes`; exact edge depends on `DAT_0089F690` runtime initialization. | `0x0073F0A0` HasBib branch; `0x0049F300` initializes `DAT_0089F690 = (1,param_2)`, consistent with east-edge if `param_2=0`; prior `BIB_SYSTEM_GHIDRA_REPORT.md`. | Severe/Frequent. Affects common base buildings and factory/refinery approach cells. |
| `NumberImpassableRows` (`BuildingType+0x1620`) has a verified helper: if `-1`, all same-building cells pass the helper; otherwise it returns true only when `cell.x < foundation_origin_x + rows`. | Conditional. Parsed for stock refineries, war factories, repair depots, naval yards, and CAOUTP; full UnitClass branch reachability per type is not exhausted. | `FUN_00458A00 @ 0x00458A00`; parser read in `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; INI stock entries. | Severe/Frequent if omitted. These are high-traffic structures; Rust parses the field but does not consume it in blocker generation. |

## 3. Current Rust Comparison

| Rust surface | Current behavior | Discrepancy sizing |
|---|---|---|
| `src/sim/production/production_tech.rs::building_footprint_cells` | Builds one adjusted footprint: base rectangle plus `AddOccupy`, minus `RemoveOccupy`. | Wrong model. gamemd keeps base foundation and hidden occupancy modifiers separate. |
| `src/sim/production/production_tech.rs::building_movement_blocking_cells` | For `Bib=yes`, drops cells with no east neighbor in the already adjusted footprint. | Partly matches only the inferred east-edge HasBib case; wrong when add/remove outliers change which cells count as an edge. |
| `src/sim/pathfinding/core.rs::PathGrid::block_building_footprint` | Hard-blocks the adjusted/bib-filtered cells in the terrain grid. | Too strict for `AddOccupy`; too loose for `RemoveOccupy`; does not model `NumberImpassableRows`. |
| `src/app_init.rs` and `src/app_sim_tick.rs` | Rebuild the grid from map/sim structures using the same adjusted/bib-filtered blocker function. | The discrepancy applies both at load and every sim tick. |
| `src/sim/movement/bump_crush.rs::build_entity_block_sets` | Builds dynamic entity blockers with the same adjusted/bib-filtered cells. | Movement replans and blocked-cell handling inherit the same wrong cells. |

## 4. Stock Structure Impact

| Structure class | Stock examples | INI facts | Likely Rust direction | Severity |
|---|---|---|---|---|
| War factories | `GAWEAP`, `NAWEAP`, `YAWEAP` | `Bib=yes`, `NumberImpassableRows=1`; art has multiple `RemoveOccupy` entries and some add/remove variants. | Mixed but high risk: Rust may allow cells because of RemoveOccupy while binary still has base content plus HasBib/row logic; Rust also ignores row count. | Severe/Frequent. Vehicle factories are built in nearly every match and vehicle exits cross these cells. |
| Refineries | `GAREFN`, `NAREFN` | `Bib=yes`, `NumberImpassableRows=3`; `GAREFN AddOccupy=-1,0/-1,-1 RemoveOccupy=3,1`; `NAREFN` has many RemoveOccupy, mostly outside base plus `3,1`. | Rust is too strict on GAREFN west AddOccupy visual cells and may be accidentally right on the dock pad only because east-edge bib filtering drops it. | Severe/Frequent. Harvesters repeatedly path to and from refinery cells. |
| Repair depots/naval yards | `GADEPT`, `NADEPT`, `GAYARD`, `NAYARD`, `YAYARD` | `NumberImpassableRows=1` or `3`; art often has Add/RemoveOccupy. | Rust ignores row-count semantics and instead uses adjusted footprint. | Moderate to Severe. Less frequent than refineries/factories but highly visible when used. |
| Barracks/radar/tech buildings | `GAPILE`, `NAHAND`, `YABRCK`, `GAAIRC`, `NAPSIS`, `NATECH`, `YATECH` | Many have AddOccupy/RemoveOccupy/OccupyHeight but no `Bib=yes`. | Mostly too strict for AddOccupy hidden/visual cells and too loose for RemoveOccupy base cells. | Moderate. Common around bases, but fewer repeated vehicle docking interactions. |
| Civilian/special art structures | `CAOUTP` and many city buildings | Large numbers of hidden occupancy modifiers. | Wrong only when paths route close to their visual overhangs or removed hidden cells. | Low to Moderate in skirmish; can be Severe on urban maps. |

## 5. Specific Discrepancies

### D1 - Rust treats hidden AddOccupy as hard movement blocking

Active in YR: Conditional. Binary writes AddOccupy to `CellClass+0x100` only when `CanHideThings` is true; it does not add the building to `cell+0xE4`.

Evidence: `0x005683C0` base foundation `CellClass__AddContent` loop is separate from the later AddOccupy counter increment loop.

Severity: Moderate to Severe/Frequent. Common structures such as `GAREFN`, `GAPILE`, `NAHAND`, `YABRCK`, `GAAIRC`, `GADEPT`, `NADEPT`, and naval yards have AddOccupy. Rust will route vehicles around some cells gamemd would not treat as building occupants.

### D2 - Rust treats hidden RemoveOccupy as movement unblocking

Active in YR: Conditional. Binary RemoveOccupy decrements hidden counter only; it does not remove building base content.

Evidence: `0x005683C0` and `0x005687F0`; `0x00441F60` base placement does not read RemoveOccupy; `0x0073F0A0` scans object lists.

Severity: Severe/Frequent. `GAWEAP`/`NAWEAP` use several RemoveOccupy entries inside or near their 5x3 foundations; refineries use the dock-adjacent remove cell. Letting the grid erase these cells can create paths through cells gamemd still classifies via building passability.

### D3 - Rust's Bib/east-edge filter is applied to the wrong cell set

Active in YR: Conditional. HasBib branch is live for `Bib=yes` structures; exact edge depends on `DAT_0089F690` runtime value.

Evidence: `0x0073F0A0` HasBib branch; `0x0049F300` direction-offset initializer; `BIB_SYSTEM_GHIDRA_REPORT.md`.

Severity: Severe/Frequent. Applying east-edge filtering after Add/Remove changes the edge topology. For GAREFN-shaped sets, west AddOccupy outliers and removed dock cells can be misclassified as non-blocking or blocking for reasons the binary does not use.

### D4 - Rust ignores `NumberImpassableRows` in passability/blocker generation

Active in YR: Conditional. The helper is verified; full per-building reachability from all UnitClass branches remains partial.

Evidence: `0x00458A00` returns `cell.x < foundation_origin_x + BuildingType+0x1620` for same-building cells unless the field is `-1`; stock INI sets `NumberImpassableRows` on high-traffic structures.

Severity: Unknown to Severe/Frequent. The stock set includes war factories, refineries, repair depots, naval yards, and CAOUTP. This is likely player-visible, but this slot did not exhaust the branch context enough to say whether every listed type reaches the helper during ordinary pathfinding.

## 6. Open Questions - Final State

- `[RESOLVED] OQ-1 - Do AddOccupy/RemoveOccupy mutate base building object lists? -> No; base foundation content and hidden counter writes are separate.` Evidence: `0x00441F60`, `0x005683C0`, `0x005687F0`.
- `[RESOLVED] OQ-2 - Does UnitClass direct path blocking read the hidden counter? -> Not in the decompiled central object-list branch; it scans `cell+0xE4/E8` objects.` Evidence: `0x0073F0A0`.
- `[RESOLVED] OQ-3 - Is HasBib passability real? -> Yes, conditional on `Bib=yes` and direction offset; it probes an adjacent cell and skips the current building if the same building is absent there.` Evidence: `0x0073F0A0`.
- `[RESOLVED] OQ-4 - Is the east-edge assumption proven? -> Partially: `0x0049F300` can initialize `DAT_0089F690` to an east-like `(1,0)` if `param_2=0`, but runtime call/default was not debugger-confirmed.` Evidence: `0x0049F300`; prior bib report.
- `[RESOLVED] OQ-5 - Does `NumberImpassableRows` have binary behavior beyond parsing? -> Yes, helper `0x00458A00` checks same-building cell X against foundation origin plus row count.` Evidence: `0x00458A00`.
- `[DEFERRED] OQ-6 - Which exact UnitClass branches invoke `0x00458A00` for every stock structure?` Category: bounded-cost-too-high; reason: full `UnitClass::Can_Enter_Cell` branch exhaustion is larger than this discrepancy-sizing slot; next step: focused NumberImpassableRows investigation.
- `[DEFERRED] OQ-7 - Which non-path/render consumers read `CellClass+0x100`?` Category: requires-different-system-context; reason: this slot verified writers and the central UnitClass path, not the complete hidden/occlusion consumer map; next step: xref all `CellClass+0x100` readers.

## 7. Verdict

Current Rust building blocker generation is **not merely too strict or too loose globally**. It is structurally wrong because it precomputes one adjusted blocker set where gamemd uses base object lists plus per-entry passability branches.

Highest-risk visible mismatches:

1. Severe/Frequent: factories and refineries, because they combine `Bib=yes`, `NumberImpassableRows`, and art occupancy modifiers.
2. Moderate: barracks/radar/tech/service structures with Add/RemoveOccupy but no bib.
3. Low to Moderate: civilian/special large buildings, except urban maps where routing around overhangs becomes common.

## Sources

- Ghidra decompiled: `0x0073F0A0`, `0x005683C0`, `0x005687F0`, `0x00441F60`, `0x00458A00`, `0x0047C520`, `0x0049F300`, `0x0045FE50`.
- Prior reports: `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`, `BIB_SYSTEM_GHIDRA_REPORT.md`, `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`, `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`.
- Rust checked: `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`, `src/sim/movement/bump_crush.rs`, `src/rules/object_type.rs`.
