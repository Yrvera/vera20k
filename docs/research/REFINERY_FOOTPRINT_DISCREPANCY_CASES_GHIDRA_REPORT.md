# Refinery Footprint Discrepancy Cases - Ghidra Research Report

**Target:** `REFINERY_FOOTPRINT_DISCREPANCY_CASES`  
**Investigation mode:** exhaustive-slice for stock `GAREFN`, `NAREFN`, and `YAREFN` refinery footprint cases.  
**Scope:** quantify cell-set differences caused by Rust merging `AddOccupy` / `RemoveOccupy` into the building footprint while `gamemd.exe` keeps the normal foundation list separate and applies modifiers to hidden occupancy.  
**Non-scope:** full A* ranking, full placement validator branch audit, full click/selection hit-test audit, and non-stock refinery-like buildings.  
**Coordinate convention:** all sets are relative to the placed building origin / NW foundation cell `(rx, ry)`. `(3,1)` means `(rx+3, ry+1)`.  
**Confidence:** High for INI cell sets, parser/writer split, and normal occupancy path; Medium for final unit pathing consequences because the full passability chain is covered by sibling reports, not re-drained here.

## Sources Checked

- Prior context: `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`.
- Ghidra read-only:
  - `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`: `Foundation`, `CanHideThings`, `AddOccupy%d`, `RemoveOccupy%d`, `OccupyHeight`, and foundation pointer assignment.
  - `BuildingClass__Place_OccupyMap @ 0x00441F60`: walks the foundation cell list from vtable `+0x108`, not add/remove lists.
  - `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`: adds base foundation contents, then hidden occupancy height/add/remove counters behind `CanHideThings`.
  - `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`: removes base foundation contents and reverses hidden occupancy height/add effects.
  - `BuildingTypeClass__GetFoundationWidth @ 0x0045EC90`, `GetFoundationHeight @ 0x0045ECA0`.
- INI:
  - `ini/artmd.ini:1706..1760` `[NAREFN]`.
  - `ini/artmd.ini:1763..1795` `[GAREFN]`.
  - `ini/artmd.ini:1799..1817` `[YAREFN]`.
  - `ini/rulesmd.ini:11722..11767`, `12515..12558`, `13234..13303`.
- Rust read-only:
  - `src/sim/production/production_tech.rs:566..647`.
  - `src/sim/pathfinding/core.rs:1461..1488`.
  - `src/sim/world/world_spawn.rs:242..247`, `430..435`.

## Binary Ground Truth

### Finding 1 - Add/Remove do not alter the normal foundation list

**Active in YR:** Yes. Evidence: `BuildingTypeClass_ReadINI_Water @ 0x0046152C..0x00461541` assigns `BuildingTypeClass+0xDFC` from `0x0089C900 + foundation_id * 120`; `AddOccupy%d` / `RemoveOccupy%d` are parsed earlier but are not inputs to this pointer calculation. `BuildingClass__Place_OccupyMap @ 0x00441F60` then walks vtable `+0x108` foundation deltas and marks those cells.

**Player-visible impact:** placement and normal building occupancy use the base foundation cells. Rust systems that use the merged footprint can block placement on cells gamemd would allow, or allow placement on a base foundation cell gamemd would block.

### Finding 2 - Add/Remove apply to hidden occupancy counters only

**Active in YR:** Conditional. Evidence: `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` first adds the object to each base foundation cell content list. Only after `WhatAmI()==building` and `BuildingTypeClass+0x1766 CanHideThings` does it update `CellClass+0x100` using `max(OccupyHeight - 1, 1)`, `AddOccupy` increments, and `RemoveOccupy` nonzero decrements. `ExitCell @ 0x005687F0` reverses height and add increments. Stock `GAREFN` and `NAREFN` set `CanHideThings=true`; stock `YAREFN` sets `CanHideThings=False`.

**Player-visible impact:** these cells affect behind-building hidden occupancy where the counter is consumed. This slot did not prove that `CellClass+0x100` directly blocks unit pathing.

### Finding 3 - Rust currently merges Add/Remove into the authoritative footprint

**Active in YR:** No, this is Rust behavior. Evidence: `building_footprint_cells` builds rectangle cells, inserts every `add_occupy`, removes every `remove_occupy`, and callers use the result for structure occupancy and path blocking (`production_tech.rs:576..613`, `world_spawn.rs:242..247`, `430..435`, `pathfinding/core.rs:1468..1488`).

**Player-visible impact:** frequent around Allied/Soviet refineries because these are common economy buildings and every player paths harvesters and vehicles around them.

## Stock Refinery Cell Sets

### `GAREFN` Allied Ore Refinery

INI: `Foundation=4x3`, `QueueingCell=4,1`, `CanHideThings=True`, `OccupyHeight=2`, `AddOccupy1=-1,0`, `AddOccupy2=-1,-1`, `RemoveOccupy1=3,1`, `Bib=yes`, `DockUnload=yes`, `NumberOfDocks=1`.

**Active in YR:** Yes for base foundation and docking data; hidden occupancy conditional and active because `CanHideThings=True`.

- Base foundation / gamemd normal occupancy and placement:
  `{(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2),(3,0),(3,1),(3,2)}`
- Rust merged footprint:
  `{(-1,-1),(-1,0),(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2),(3,0),(3,2)}`
- Difference, Rust versus gamemd normal foundation:
  - Extra Rust footprint cells: `{(-1,-1),(-1,0)}`
  - Missing Rust footprint cell: `{(3,1)}`
- Expected gamemd hidden occupancy effect:
  - Height depth is `max(2 - 1, 1) = 1`, so base hidden increments are exactly the base foundation cells.
  - Add increments: `{(-1,0),(-1,-1)}`
  - Remove decrements if nonzero: `{(3,1)}`
  - Net hidden-positive set equals the Rust merged footprint, but this is hidden occupancy, not normal foundation occupancy.

**Discrepancy severity:** Severe/Frequent. GAREFN is built in nearly every Allied game. Rust treats `(-1,0)` and `(-1,-1)` as real structure footprint cells and omits `(3,1)` from placement/occupancy, while gamemd only applies those changes to hidden occupancy. Common symptoms: placement can be rejected west/northwest of the refinery when gamemd would allow it, and the dock pad `(3,1)` is not represented as a normal occupied foundation cell in Rust.

**Unit path blocking around refinery:** Moderate/Frequent. With `Bib=yes`, sibling bib research verifies gamemd relaxes the east-edge occupant block, so base cells `(3,0),(3,1),(3,2)` are passable to units even though they are still normal foundation cells. Rust's merged footprint plus bib filter yields blockers `{(-1,0),(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,2)}`. Compared to gamemd's base-bib blocker set `{(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2)}`, Rust incorrectly blocks `(-1,0)` and incorrectly relaxes `(2,1)`.

**Docking / harvester approach:** Moderate/Frequent. The stock queue cell `(4,1)` is outside both sets. The dock pad `(3,1)` is inside the base foundation, removed from Rust merged footprint, and has hidden occupancy canceled in gamemd by `RemoveOccupy1`. The final pad being passable is broadly aligned, but Rust gets there by deleting it from the footprint, while gamemd keeps it as a foundation cell and relies on per-cell passability / hidden-occupancy rules.

**Selection / hit footprint:** Unknown. This slot verified foundation width/height and normal occupancy paths, but did not audit tactical click hit-tests. Existing Rust selection/brackets use foundation dimensions, not merged cells, so no concrete refinery selection discrepancy is claimed here.

### `NAREFN` Soviet Ore Refinery

INI: `Foundation=4x3`, `QueueingCell=4,1`, `CanHideThings=true`, `OccupyHeight=4`, `RemoveOccupy1=0,-2`, `RemoveOccupy2=1,-1`, `RemoveOccupy3=1,-2`, `RemoveOccupy4=2,-1`, `RemoveOccupy5=-2,0`, `RemoveOccupy6=-2,-1`, `RemoveOccupy7=-2,-2`, `RemoveOccupy8=3,1`, `Bib=yes`, `DockUnload=yes`, `NumberOfDocks=1`.

**Active in YR:** Yes for base foundation and docking data; hidden occupancy conditional and active because `CanHideThings=true`.

- Base foundation / gamemd normal occupancy and placement:
  `{(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2),(3,0),(3,1),(3,2)}`
- Rust merged footprint:
  `{(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2),(3,0),(3,2)}`
- Difference, Rust versus gamemd normal foundation:
  - Extra Rust footprint cells: `{}`
  - Missing Rust footprint cell: `{(3,1)}`
- Expected gamemd hidden occupancy effect:
  - Height depth is `max(4 - 1, 1) = 3`.
  - Height union before removes:
    `{(-2,-2),(-2,-1),(-2,0),(-1,-2),(-1,-1),(-1,0),(-1,1),(0,-2),(0,-1),(0,0),(0,1),(0,2),(1,-2),(1,-1),(1,0),(1,1),(1,2),(2,-1),(2,0),(2,1),(2,2),(3,0),(3,1),(3,2)}`
  - Remove decrements if nonzero:
    `{(0,-2),(1,-1),(1,-2),(2,-1),(-2,0),(-2,-1),(-2,-2),(3,1)}`
  - Net hidden-positive set:
    `{(-1,-2),(-1,-1),(-1,0),(-1,1),(0,-1),(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2),(3,0),(3,2)}`

**Discrepancy severity:** Severe/Frequent. NAREFN is the stock Soviet economy building. Rust only removes `(3,1)` from a 4x3 footprint; gamemd keeps `(3,1)` in the normal foundation and uses the eight remove offsets to carve hidden occupancy from a taller diagonal hidden set. Rust therefore misses the whole north/northwest hidden-occupancy shape and misclassifies the dock pad for placement/normal occupancy.

**Unit path blocking around refinery:** Moderate/Frequent. Gamemd base-bib blocker set is `{(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2)}`. Rust merged-footprint bib blockers are `{(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,2)}`. Rust incorrectly relaxes `(2,1)` because deleting `(3,1)` makes `(2,1)` look like an east edge of the shape.

**Docking / harvester approach:** Moderate/Frequent. Queue cell `(4,1)` is unchanged. Dock pad `(3,1)` is a gamemd base foundation cell but has hidden occupancy canceled by `RemoveOccupy8`; Rust deletes it from the structure footprint. As with GAREFN, passability may look similar at the pad, but the occupancy model differs.

**Selection / hit footprint:** Unknown. Not audited beyond width/height and foundation use.

### `YAREFN` Yuri Ore Refinery / deployed Slave Miner

INI: `Foundation=2x2`, `CanHideThings=False`, `OccupyHeight=2`, no `AddOccupy*`, no `RemoveOccupy*`, no `Bib=yes`, no `DockUnload=yes`, no `NumberOfDocks=`.

**Active in YR:** Yes for base foundation; hidden occupancy modifier path is inactive because stock art sets `CanHideThings=False`. The building is a live Yuri resource building, but it is not the same stock dock-unload refinery pattern as GAREFN/NAREFN.

- Base foundation / gamemd normal occupancy and placement:
  `{(0,0),(0,1),(1,0),(1,1)}`
- Rust merged footprint:
  `{(0,0),(0,1),(1,0),(1,1)}`
- Difference, Rust versus gamemd normal foundation:
  - Extra Rust footprint cells: `{}`
  - Missing Rust footprint cells: `{}`
- Expected gamemd hidden occupancy effect:
  - None from height/add/remove, because `CanHideThings=False` gates the hidden occupancy block.

**Discrepancy severity:** Low. For this specific Add/Remove merge issue, stock YAREFN has no cell-set discrepancy. Any Yuri slave-miner deploy/path discrepancy is outside this target.

## Concrete Player-Visible Surfaces

| Surface | GAREFN | NAREFN | YAREFN |
|---|---|---|---|
| Placement blocking | Severe/Frequent: Rust wrongly treats `(-1,-1),(-1,0)` as occupied and fails to treat `(3,1)` as base foundation. Active in YR: Yes. | Severe/Frequent: Rust wrongly fails to treat `(3,1)` as base foundation. Active in YR: Yes. | Low: no set difference. Active in YR: Yes. |
| Unit path blocking | Moderate/Frequent: Rust blocks `(-1,0)` and relaxes `(2,1)` relative to gamemd base-bib blocker set. Active in YR: Conditional on ground units checking building-occupied cells; standard YR path. | Moderate/Frequent: Rust relaxes `(2,1)` relative to gamemd base-bib blocker set. Active in YR: Conditional on ground units checking building-occupied cells; standard YR path. | Low: no Add/Remove discrepancy. |
| Dock/approach cells | Moderate/Frequent: queue `(4,1)` unaffected; pad `(3,1)` model differs. Active in YR: Yes for stock dock-unload refinery. | Moderate/Frequent: queue `(4,1)` unaffected; pad `(3,1)` model differs. Active in YR: Yes for stock dock-unload refinery. | Low/Conditional: no stock dock-unload data in checked rules/art. |
| Hidden occupancy / behind-building hiding | Moderate: net hidden set equals Rust footprint only by coincidence because `OccupyHeight=2`. Active in YR: Conditional and true for GAREFN. | Severe/Frequent visually near Soviet refineries: hidden occupancy has a 16-cell net diagonal shape, not Rust's 11-cell footprint. Active in YR: Conditional and true for NAREFN. | Low: gated off by `CanHideThings=False`. |
| Selection/hit footprint | Unknown: not audited. Existing Rust selection dimensions likely avoid this specific merge. Active in YR: unknown consumer. | Unknown. | Unknown. |

## Load-Bearing Facts

1. `Place_OccupyMap @ 0x00441F60` uses the base foundation list and never reads `BuildingTypeClass+0x1624..0x16A0` Add/Remove pairs. Active in YR: Yes.
2. `EnterCell @ 0x005683C0` and `ExitCell @ 0x005687F0` apply Add/Remove only inside the `CanHideThings` hidden occupancy block. Active in YR: Conditional; true for `GAREFN`/`NAREFN`, false for `YAREFN`.
3. Stock `GAREFN` and `NAREFN` are both `Foundation=4x3`, `Bib=yes`, `DockUnload=yes`, `NumberOfDocks=1`, `QueueingCell=4,1`; stock `YAREFN` is `Foundation=2x2`, `CanHideThings=False`, no Add/Remove keys. Active in YR: Yes, from `artmd.ini`/`rulesmd.ini`.
4. Rust `building_footprint_cells` merges Add/Remove into one set and callers use it for occupancy/pathing. Active in YR: No; Rust-only mismatch.
5. NAREFN's expected gamemd hidden net is 16 cells after height-depth 3 and eight removes; Rust's merged footprint is 11 cells. Active in YR: Conditional and true for stock NAREFN because `CanHideThings=true`.

## Open Questions

- Which exact UI/tactical click hit-test paths use foundation dimensions versus cell content lookup for buildings? Deferred; not needed for this placement/path/hidden-occupancy discrepancy sizing.
- Which render or targeting systems consume `CellClass+0x100` besides behind-object hiding? Deferred to a dedicated hidden-occupancy consumer audit.

## Status

COMPLETE for the requested stock refinery discrepancy sizing.
