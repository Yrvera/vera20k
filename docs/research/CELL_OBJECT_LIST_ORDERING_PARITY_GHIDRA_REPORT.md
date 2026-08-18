# Cell Object List Ordering Parity - Ghidra Research Report

**Address(es):** `0x0047E8A0`, `0x0047EA90`, `0x0073F0A0`, `0x00429830`, `0x00481670`, `0x00489280`, `0x0047C3D0`, `0x0047C520`, `0x0047DD70`
**Investigation Mode:** coverage-map
**Claimed Scope:** `CellClass::AddContent` / `RemoveContent` ordering semantics, verified first/iteration consumers, and current Rust ordering implications.
**Non-Scope:** hidden `CellClass+0x100` occupancy readers; full `UnitClass::Can_Enter_Cell` port; save/load serialization order; target acquisition outside known cell-list helpers.
**Confidence:** High for linked-list insertion/removal and named consumers already verified by prior Ghidra reports; Medium for Rust consumer completeness because no fresh Ghidra MCP was exposed in this session.
**Active in YR:** Yes for normal cell-list maintenance, movement/pathing, area damage, scatter, nearest-object lookup, and bridge collapse; conditional for bridge/deck list behavior on bridge-cell and `OnBridge` state.

## 1. Overview

`gamemd.exe` stores each `CellClass` object list as two singly-linked lists: ground `FirstObject` and bridge/alternate `AltObject`. Ordering is not a pure set property: buildings append to the selected list tail when the list is non-empty, while units, infantry, aircraft, and most other non-buildings prepend to the selected list head. Several live YR consumers then walk head-to-tail or keep the first matching/tied object, so insertion order can become player-visible in passability, movement cost, scatter, area damage side effects, and object lookup.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose | Evidence | Active in YR |
|---:|---|---|---|---|
| `CellClass+0xE4` | `ObjectClass*` | Ground object-list head (`FirstObject`) | `CellClass::AddContent @ 0x0047E8A0`; `RemoveContent @ 0x0047EA90`; `UnitClass::Can_Enter_Cell @ 0x0073F0A0`; `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` | Yes |
| `CellClass+0xE8` | `ObjectClass*` | Bridge/deck alternate object-list head (`AltObject`) | same functions and report | Conditional: used when selected list flag / bridge traversal chooses deck list |
| `ObjectClass+0x30` | pointer | next pointer in selected cell list | `AddContent` / `RemoveContent`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` | Yes |
| `ObjectClass+0x8C` | byte | normal list-layer source passed by Techno enter/exit cell helpers | callsites `0x005684BB` / `0x005688EB`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` | Yes |

## 3. Core Logic

`CellClass::AddContent @ 0x0047E8A0` selects only one list from its stack list-layer argument: zero selects `+0xE4`, nonzero selects `+0xE8`. Active in YR: Yes; Techno enter-cell code calls this in normal movement and spawn/list registration paths.

After list selection, `AddContent` treats `WhatAmI()==6` specially. Verified `WhatAmI` mapping from the follow-up report is `Unit=1`, `Aircraft=2`, `Building=6`, `Infantry=0xF`; therefore only buildings/structures take the append path among current Rust categories. Active in YR: Yes.

Building insertion appends only when the selected list is already non-empty. A building added to an empty list becomes the head through the normal path, and later non-buildings can still prepend in front of it. Active in YR: Yes.

Non-buildings prepend to the first occupant position of the selected list. This makes recent mobile entrants scan before older occupants on the same ground/bridge list. Active in YR: Yes.

`CellClass::RemoveContent @ 0x0047EA90` uses the same selected-list argument, unlinks only from that list, clears the removed object's `+0x30`, and preserves relative order of remaining occupants. Active in YR: Yes.

Prior bridge reports add one important duplicate/removal nuance: AddContent has only a narrow selected-head duplicate guard, not a full cross-list scan, and RemoveContent does not search the other layer. Active in YR: Yes.

## 4. INI Keys

No INI key controls the `CellClass` object-list ordering rule. YR `rules*.ini` / `art*.ini` can affect which objects exist, object type, foundation, bridge rules, and warhead behavior, but the append-vs-prepend list rule is binary code at `0x0047E8A0`. Active in YR: Yes.

## 5. Integration Points

| Consumer | Ordering-sensitive behavior | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Can_Enter_Cell @ 0x0073F0A0` | Selects one list and walks head-to-tail; loop mixes early returns with accumulated result codes. | `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` | Yes |
| `AStar_compute_edge_cost @ 0x00429830` | For moving-friendly code, predicts the selected list head object, with a 10-cell lookahead. | `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md` | Yes |
| `CellClass::Scatter_Objects @ 0x00481670` | Collects selected-list occupants and scatters in collected order; prior report caps selection at up to 10. | `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`; `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` | Yes |
| `Apply_area_damage @ 0x00489280` | Walks selected cell-list order while building damage target vector; no sort was observed in prior decompile. | `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`; `WARHEAD_DETONATE_GHIDRA_REPORT.md` | Yes |
| `CellClass::Find_Nearest_Object @ 0x0047C3D0` | Updates best only on strictly smaller distance, so equal-distance ties keep earlier list object. | `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md` | Yes |
| `Look_up_building_in_cell @ 0x0047C520` | Scans ground list and returns first `WhatAmI()==6`. | `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` | Yes |
| `CellClass::BlowUpBridge @ 0x0047DD70` | Walks ground list for C4Warhead death and bridge list for `DropIn`; each loop snapshots next before side effect. | `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` | Conditional: bridge collapse |

Click/hover selection in current Rust is not currently a CellClass list consumer: `src/app_entity_pick.rs` scans `EntityStore::values()` and picks by screen/foundation distance. That may be correct for the app's current selection approximation, but no binary finding in this slot proves retail click selection uses `CellClass+0xE4/+0xE8` list order. Active in YR: deferred, not claimed.

## 6. Current Rust Implementation Status

Current Rust already implements the core list-order rule in `src/sim/occupancy.rs`: `CellListInsertion::from_category` maps structures to append and all other categories to prepend (`src/sim/occupancy.rs:30`, `src/sim/occupancy.rs:36`), `iter_layer` exposes selected-layer iteration (`src/sim/occupancy.rs:54`), `add` inserts before the first same-layer occupant for non-buildings and after the last same-layer occupant for buildings (`src/sim/occupancy.rs:161`, `src/sim/occupancy.rs:169`), and `remove` retains relative order (`src/sim/occupancy.rs:182`).

Current Rust also models the normal list-layer source through `GameEntity::occupancy_list_layer`, using `on_bridge` rather than locomotor layer for ground/bridge list membership (`src/sim/game_entity.rs:452`, `src/sim/game_entity.rs:458`). `OccupancyGrid::rebuild` calls that method and then applies `CellListInsertion::from_category` (`src/sim/occupancy.rs:110`, `src/sim/occupancy.rs:117`, `src/sim/occupancy.rs:128`).

Known Rust consumers that already observe the gamemd-style order include `pathfinding::cell_entry::find_primary_blocker`, which iterates `occ.iter_layer(layer)` and returns the first usable occupant (`src/sim/pathfinding/cell_entry.rs:398`, `src/sim/pathfinding/cell_entry.rs:407`), and `combat_aoe::apply_aoe_damage`, which iterates the selected layer in `iter_layer` order while deduplicating targets (`src/sim/combat/combat_aoe.rs:69`, `src/sim/combat/combat_aoe.rs:85`).

Remaining Rust risk: some helpers still use category-specific split iterators (`blockers()` then `infantry()`), which preserves order within each category but not necessarily a single combined CellClass scan. Examples include boolean crush passability (`src/sim/movement/bump_crush.rs:513`, `src/sim/movement/bump_crush.rs:528`) and idle-scatter diagnostics/logging (`src/sim/movement/scatter.rs:154`). These may be correct if the local semantic is category/count based, but they should not be treated as a general replacement for CellClass list iteration.

Bridge collapse Rust now relayers dropped deck entities by clearing `on_bridge`, snapping to ground, and calling `sim.occupancy.move_entity(... MovementLayer::Ground ...)` (`src/sim/world/bridge_orchestrator.rs:1012`, `src/sim/world/bridge_orchestrator.rs:1033`, `src/sim/world/bridge_orchestrator.rs:1050`). Ground-occupant killing currently gathers victims from `EntityStore::iter_sorted()` rather than from selected ground list order (`src/sim/world/bridge_orchestrator.rs:749`, `src/sim/world/bridge_orchestrator.rs:752`). If death side-effect order becomes observable, this needs a CellClass-order test.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::AddContent` selected-list choice | verified | `0x0047E8A0`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md` | none |
| `AddContent` building append vs non-building prepend | verified | `0x0047E8A0`; WhatAmI table in follow-up report | none |
| `CellClass::RemoveContent` order-preserving unlink | verified | `0x0047EA90`; prior reports | none |
| Techno add/remove layer argument source | verified | `0x005684BB`, `0x005688EB`; `ObjectClass+0x8C` | non-Techno writers deferred |
| `UnitClass::Can_Enter_Cell` list-order sensitivity | verified | `0x0073F0A0`; follow-up report | full direct Rust port out of scope |
| A* moving-friendly selected-head consumer | verified | `0x00429830`; ordering report | exact Rust A* cost parity outside this slice |
| Scatter list-order consumer | verified | `0x00481670`; ordering report | full scatter behavior not rechecked |
| Area-damage list-order discovery | verified | `0x00489280`; ordering report | exact side-effect order tests in Rust |
| Click/hover selection | deferred | `src/app_entity_pick.rs:390` scans entities by distance | needs separate retail click-selection investigation |
| Hidden `CellClass+0x100` readers | deferred | parent assigned slot 2 | out of scope by instruction |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-COL-001 - Which lists exist? -> CellClass has separate ground `+0xE4` and bridge/deck `+0xE8` object lists.` (evidence: `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`; `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-COL-002 - Does AddContent append buildings? -> Yes, only `WhatAmI()==6` and only when selected list is non-empty.` (evidence: `0x0047E8A0`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-COL-003 - Does AddContent prepend non-buildings? -> Yes, units, aircraft, infantry, and other non-`6` objects prepend to selected list head.` (evidence: `0x0047E8A0`; WhatAmI table)
- `[RESOLVED] OQ-COL-004 - Does RemoveContent preserve remaining order? -> Yes, it unlinks target from the selected list and clears target `+0x30`.` (evidence: `0x0047EA90`)
- `[RESOLVED] OQ-COL-005 - Is the layer recomputed inside AddContent/RemoveContent? -> No, caller passes list-layer argument, normally from `ObjectClass+0x8C`.` (evidence: `0x005684BB`, `0x005688EB`)
- `[RESOLVED] OQ-COL-006 - Where is first/iteration order player-visible? -> Can-enter, A* moving-friendly cost, scatter, area damage, nearest-object ties, building lookup, bridge collapse.` (evidence: addresses in Section 5)
- `[RESOLVED] OQ-COL-007 - Does current Rust still append every occupant? -> No; current `OccupancyGrid::add` has prepend/append insertion classes.` (evidence: `src/sim/occupancy.rs:30`, `src/sim/occupancy.rs:161`, `src/sim/occupancy.rs:169`)
- `[RESOLVED] OQ-COL-008 - Does current Rust have an order-preserving selected-layer iterator? -> Yes, `CellOccupancy::iter_layer`.` (evidence: `src/sim/occupancy.rs:54`)
- `[RESOLVED] OQ-COL-009 - Does current Rust pathfinding primary blocker use combined selected-list order? -> Yes for this helper.` (evidence: `src/sim/pathfinding/cell_entry.rs:407`)
- `[DEFERRED] OQ-COL-010 - Does retail click selection use CellClass list order?` (category: `requires-different-system-context`; reason: current slot found Rust distance/stable-id picking but no scoped Ghidra evidence for retail click selection; next-step-if-pursued: investigate TacticalClass/ObjectClass screen-pick routines)
- `[DEFERRED] OQ-COL-011 - Save/load exact cell-list reconstruction order.` (category: `requires-different-system-context`; reason: not required to confirm AddContent/RemoveContent semantics; next-step-if-pursued: trace serialization/load object registration)
- `[DEFERRED] OQ-COL-012 - Hidden `CellClass+0x100` readers.` (category: `out-of-scope`; reason: explicitly assigned to slot 2; next-step-if-pursued: consume slot-2 report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Structures append to the selected ground/bridge list; non-structures prepend; RemoveContent preserves order. | `0x0047E8A0`, `0x0047EA90`; `src/sim/occupancy.rs:161`, `src/sim/occupancy.rs:169` | none observed for core occupancy add/remove | `src/sim/occupancy.rs` | Keep insertion class explicit and preserve selected-layer order across add/move/remove/rebuild. | Spawn building in occupied ground cell, then move two units into same cell; expected selected-layer iteration is newest unit, older unit, building. Proposed test: `test_cell_occupancy_iteration_preserves_building_tail_order`. | Do not replace with stable-id sort or append-only Vec rebuild. |
| First-match consumers must use one combined selected-list order, not blockers-before-infantry category priority, when matching a CellClass scan. | `UnitClass::Can_Enter_Cell @ 0x0073F0A0`; `CellClass::Find_Nearest_Object @ 0x0047C3D0`; Rust `find_primary_blocker @ src/sim/pathfinding/cell_entry.rs:407` | mostly implemented for primary blocker; unchecked for every consumer | `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/movement/scatter.rs`, future targeting helpers | Consumers that are intended to mirror a binary list scan should iterate `iter_layer` once and apply predicates in order. | Put same-layer infantry and vehicle in one cell where the infantry was added later; primary blocker/target tie should pick the infantry if binary scan would. Proposed test: `test_cell_list_order_primary_blocker_prefers_newer_infantry_over_older_vehicle`. | Do not infer "vehicles before infantry" from separate helper names. |
| Bridge collapse treats ground and bridge lists differently: ground-list occupants die via C4Warhead path, bridge-list occupants `DropIn`; both loops are list walks with next snapshot before side effect. | `CellClass::BlowUpBridge @ 0x0047DD70`; Rust `kill_ground_occupants_at @ src/sim/world/bridge_orchestrator.rs:749`; `drop_in_bridge_deck_entities @ src/sim/world/bridge_orchestrator.rs:1012` | outcome mostly represented; side-effect order for ground deaths is sorted-store, not proven list-order | `src/sim/world/bridge_orchestrator.rs` | If death/drop side-effect order matters, collect victims from occupancy selected list rather than `EntityStore::iter_sorted()`. | Ground and deck occupants share bridge 2D cell; collapse kills only ground and relayers only deck, preserving deterministic list-order event log. Proposed test: `test_bridge_collapse_walks_ground_and_deck_lists_in_cell_order`. | Do not merge ground/deck occupants into a single kill/drop set. |

### Negative Facts / Do Not Do

- Do not implement one unordered per-cell set and sort by stable id for gameplay iteration. Evidence: binary traverses `ObjectClass+0x30` linked-list order in `UnitClass::Can_Enter_Cell @ 0x0073F0A0` and other consumers. Active in YR: Yes.
- Do not append all Rust occupants. Evidence: `CellClass::AddContent @ 0x0047E8A0` prepends non-`WhatAmI()==6` objects. Active in YR: Yes.
- Do not append every `Structure` ahead of existing mobiles. Evidence: buildings append only after the selected list's current tail; later non-buildings can still prepend in front. Active in YR: Yes.
- Do not remove an object from both ground and bridge lists to be "safe". Evidence: `CellClass::RemoveContent @ 0x0047EA90` selects only one list from the argument; prior bridge reports note no cross-list scan. Active in YR: Yes.
- Do not collapse high-bridge deck and ground occupants into a single blocking list. Evidence: `+0xE4/+0xE8` are independently selected and `BlowUpBridge @ 0x0047DD70` applies different effects to each. Active in YR: Conditional on bridge cells, but live in standard YR bridge play.

### Stale Docs / Follow-up Docs

- `docs/research/COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md` Section "Current Rust implications" should replace "Rust vector/list order may differ and affect targeting/passability edge cases if not normalized" with: "Current Rust `OccupancyGrid` has an explicit `CellListInsertion` model matching verified gamemd structure-vs-non-structure insertion, but every first-match consumer still needs audit: use `CellOccupancy::iter_layer` for binary-style CellClass scans, and document any consumer that intentionally uses category/count semantics."
- `docs/research/CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md` Section 4.1 should replace "Current `OccupancyGrid::add` appends every occupant" with: "Current `OccupancyGrid::add` accepts `CellListInsertion` and inserts non-buildings before the first same-layer occupant while inserting buildings after the last same-layer occupant."

## Sources

- Existing verified Ghidra reports: `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`; `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`; `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`; `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`; `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`; `WARHEAD_DETONATE_GHIDRA_REPORT.md`.
- Addresses cited by prior Ghidra reports: `0x0047E8A0`, `0x0047EA90`, `0x005684BB`, `0x005688EB`, `0x0073F0A0`, `0x00429830`, `0x00481670`, `0x00489280`, `0x0047C3D0`, `0x0047C520`, `0x0047DD70`.
- Rust files scanned read-only: `src/sim/occupancy.rs`, `src/sim/game_entity.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/combat/combat_aoe.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/movement/scatter.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/app_entity_pick.rs`.
