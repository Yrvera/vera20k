# CellClass Substrate First Migration Slice - Handoff Plan

**Target question:** What is the first safe Rust-native migration slice for a native CellClass substrate that does not break movement, production spawn, placement, bridges, or AI all at once?
**Non-goals:** Do not implement Rust; do not rediscover broad binary behavior; do not produce a whole CellClass rewrite; do not duplicate the full Rust caller inventory owned by slot 4.
**Evidence needed to mark COMPLETE:** Existing binary-backed docs must prove the CellClass object-list, CellRect validator, bridge/layer, and lifecycle facts that the slice must preserve; focused Rust scans must show concrete current APIs and tests to extend.
**Stop conditions:** Stop before any Rust edit; stop if the first slice requires changing movement, production spawn, placement, bridges, and AI in one patch; stop if a claim cannot be tied to an existing research doc line or Rust source line.

**Mode:** coverage-map for an implementation bridge plan. No fresh Ghidra was used in this slot; binary facts are sourced to existing verified reports.

## Executive Result

The first safe migration slice is a substrate-query and test slice, not a caller migration. Keep `OccupancyGrid` as the current object-list substrate, add a narrow read-only CellRect-style substrate API around cell-field blockers, reservation bits, playfield corners, and selected object-list scans, then prove it with unit tests before changing production spawn, placement, movement, bridges, or AI callers.

This is safe because current Rust already models the core `CellClass+0xE4/+0xE8` list order: `CellListInsertion::from_category` maps structures to append and non-structures to prepend, `iter_layer` filters selected-layer order, and add/remove preserve list order (`src/sim/occupancy.rs:30`, `src/sim/occupancy.rs:36`, `src/sim/occupancy.rs:53`, `src/sim/occupancy.rs:145`, `src/sim/occupancy.rs:180`). Existing research says that rule is binary-owned and player-visible through consumers such as `Can_Enter_Cell`, A* cost prediction, scatter, area damage, nearest-object, and bridge collapse (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:12`, `:45`, `:46`, `:47`, `:48`, `:49`, `:51`).

The first slice must not replace `OccupancyGrid` with a new broad container. `OccupancyGrid` already states it is equivalent to `FirstObject`/`AltObject` style lists and remains in `sim/` with no render/UI dependency (`src/sim/occupancy.rs:7`, `:8`, `:10`, `:11`, `:12`). The missing substrate piece is the CellRect/CellClass cell-field side: current `OccupancyGrid` tracks dynamic entity occupancy but not `CellClass+0xDC`, `+0x44`, `+0x4C`, `+0x11C`, RTTI helper blockers, or final playfield-corner semantics (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:127`, `:128`, `:130`).

## Current Rust Boundary To Preserve

- `OccupancyGrid` stores per-cell occupants in a `BTreeMap` and exposes layer-filtered iteration (`src/sim/occupancy.rs:97`, `:98`, `:53`, `:54`, `:55`). This matches the Rust architecture rule: deterministic storage without an ECS crate.
- `CellListInsertion::from_category` is already the correct list insertion selector for current categories: structures append, non-structures prepend (`src/sim/occupancy.rs:30`, `:36`, `:37`, `:38`, `:39`, `:40`), matching the binary list rule (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:25`, `:27`, `:29`, `:31`).
- `GameEntity::occupancy_list_layer()` is the intended Rust analog for `ObjectClass+0x8C`/`OnBridge`, and it is deliberately separate from locomotor/path layer because ramps can disagree for a tick (`src/sim/game_entity.rs:576`, `:578`, `:579`, `:580`, `:581`, `:596`, `:597`, `:598`, `:599`).
- Save/load already rebuilds skipped occupancy from entity positions after deserialization (`src/sim/world/mod.rs:944`, `:963`, `:968`, `:971`, `:973`), but the report does not prove this rebuild order is byte-identical to gamemd save/load.
- Movement already calls `occupancy.move_entity` at cell transition time and then reserves destination/subcell state (`src/sim/movement/movement_tick.rs:1246`, `:1247`, `:1253`, `:1255`, `:1257`, `:1271`, `:1273`), so caller rewiring has high blast radius and should be delayed until the substrate tests are in place.

## First Safe Slice

1. Add a Rust-native CellClass substrate facade in `sim`, likely adjacent to `src/sim/occupancy.rs`, that is read-only over current `OccupancyGrid` plus minimal per-cell substrate fields. It should expose selected-list iteration, `check_occupancy_rect(rect, reservation_arg)`, and later `check_passability_rect(...)` without taking ownership of movement or production behavior. Evidence: CellRect passability and occupancy are independent validators (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:12`, `:14`), and `CheckOccupancy` reads cell-field/object-list blockers distinct from `OccupancyGrid` (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:93`, `:94`, `:95`, `:96`, `:97`, `:98`, `:128`).

2. Add tests at the substrate boundary first. These tests should be pure sim tests and must not require changing movement, production spawn, placement, bridge collapse, or AI call sites. Evidence: production spawn currently uses preferred offsets, `PathGrid`, and `OccupancyGrid` directly (`src/sim/production/production_spawn.rs:101`, `:104`, `:110`, `:111`, `:123`, `:128`, `:135`, `:250`, `:255`), while the validator report says production spawn/nearby fallback does not yet expose the binary `Find_Nearby` candidate flags or CellRect contracts (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:130`).

3. Only after those tests pass, migrate one low-risk caller to consume the read-only facade. The best first caller is a nearby/spawn fallback predicate behind the existing production-spawn path, because the verified FNPC shape uses both `CheckPassability` and optional `CheckOccupancy(rect, -1)` (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:116`, `:170`, `:171`), and the Rust spawn path already centralizes fallback selection through `find_spawn_cell_near_structure` (`src/sim/production/production_spawn.rs:201`, `:212`, `:217`, `:222`, `:250`).

Do not start with bridge collapse, runtime movement, or AI site placement. Bridge entry has split object-list and occupancy-bit layers that can diverge at bridgeheads (`docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md:224`, `:229`, `:231`, `:232`, `:234`), movement has same-tick list/reservation consequences (`src/sim/movement/movement_tick.rs:1246`, `:1257`, `:1271`), and AI site placement needs separate `CellClass+0xDC` reservation bits keyed by house index (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:95`, `:110`, `:117`, `:172`).

## Implementation Handoff

1. **Substrate read API and list-order guard.** Create a narrow `sim` API over `OccupancyGrid` for selected CellClass list scans and cell substrate fields, preserving current insertion/removal semantics. Acceptance scenario: in one cell, add a building, an older unit, and then a newer infantry on the same layer; selected-list iteration returns newest infantry, older unit, building. Proposed test name: `cell_substrate_selected_list_preserves_gamemd_insertion_order`. Evidence: binary list rule (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:25`, `:27`, `:29`, `:31`, `:33`); Rust implementation surface (`src/sim/occupancy.rs:145`, `:160`, `:169`, `:180`).

2. **`CheckOccupancy(rect, -1)` substrate test surface.** Add a read-only rectangle occupancy helper that skips reservation bits only for `-1`, but still rejects cell-field blockers, ground-list helper blockers, building occupants, and out-of-playfield rectangles. Acceptance scenario: an otherwise empty special/slope cell fails, a cell with only `+0xDC` reservation passes under `-1`, and an out-of-playfield rect fails through final corner checking. Proposed test name: `cell_substrate_check_occupancy_minus_one_skips_reservation_only`. Evidence: CheckOccupancy read contract (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:25`, `:93`, `:94`, `:95`, `:96`, `:97`, `:98`, `:171`).

3. **Per-house reservation surface, not caller migration.** Add a minimal reservation-bit representation or fixture-only substrate field with the exact `1 << (arg & 0x1F)` semantics, but do not wire AI placement yet. Acceptance scenario: the same rect passes with `reservation_arg = -1`, fails with the matching house index, and also fails for a non-`-1` negative value that aliases through `arg & 0x1F`. Proposed test name: `cell_substrate_reservation_arg_minus_one_vs_house_index`. Evidence: `CellClass+0xDC` semantics (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:25`, `:95`, `:154`, `:178`); occupancy-specific handoff (`docs/research/CELLRECT_CHECKOCCUPANCY_00586780_FULL_BLOCKER_TREE_GHIDRA_REPORT.md:68`, `:147`, `:148`).

## Must Remain Unchanged

- Keep `sim/` independent of render, UI, sidebar, audio, and net; `occupancy.rs` already documents that dependency boundary (`src/sim/occupancy.rs:10`, `:11`, `:12`).
- Keep object-list and occupancy-bit layers separate for Can_Enter_Cell-style movement. A single movement layer is insufficient at bridgeheads (`docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md:103`, `:114`, `:224`, `:229`, `:231`, `:232`, `:234`, `:297`).
- Keep `CheckPassability` and `CheckOccupancy` as separate concepts. `CheckPassability` calls `CheckCellPassability` with speed/zone/height/layer inputs and does not call `IsRectInPlayfield`; `CheckOccupancy` checks object/reservation/cell fields and final corner containment (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:63`, `:64`, `:70`, `:71`, `:72`, `:73`, `:75`, `:93`, `:98`).
- Keep dynamic object occupancy separate from `CellClass+0xDC` house/site reservations (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:95`, `:128`, `:172`, `:178`).
- Preserve same-state lifecycle windows: gamemd `Reveal` sets `InLimbo=0` before `Mark(PUT)`, `Conceal` removes cell membership before `InLimbo=1`, death calls `Conceal` before `IsAlive=0`, and chronoshift has an in-flight state with no cell registration (`docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md:180`, `:182`, `:186`, `:191`, `:204`, `:207`, `:219`, `:422`, `:426`, `:427`, `:345`, `:368`, `:370`, `:374`, `:375`).

## Negative Facts / Do Not Do

- Do not implement one unordered per-cell set sorted by stable id. Binary consumers walk `ObjectClass+0x30` list order, and current Rust already encodes insertion order (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:12`, `:45`, `:57`, `:107`).
- Do not remove from both ground and bridge lists as a safety measure. `RemoveContent` selects only one list, and bridge/ground lists are independent (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:33`, `:35`, `:110`, `:111`).
- Do not model `CheckOccupancy` as terrain passability or "no other units only"; it has no SpeedType/MovementZone/LandType read and does read cell-field/object-list/playfield blockers (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:93`, `:94`, `:95`, `:96`, `:97`, `:98`, `:176`).
- Do not store `CellClass+0xDC` in `OccupancyGrid` as if it were an entity. FNPC passes `-1` and skips it; AI/site helper passes a house index (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:116`, `:117`, `:171`, `:172`, `:178`).
- Do not rewrite production spawn, placement, movement, bridges, and AI in the first patch. Their current surfaces are separate and high-risk: production spawn uses `PathGrid`/`OccupancyGrid` directly (`src/sim/production/production_spawn.rs:217`, `:222`, `:225`), movement mutates occupancy mid-transition (`src/sim/movement/movement_tick.rs:1246`, `:1247`), and bridges require split layer semantics (`docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md:296`, `:297`, `:301`).

## Remaining Uncertainty

- Save/load exact CellClass object-list reconstruction order remains deferred in the object-list report (`docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:94`). Current Rust rebuilds occupancy from `EntityStore::values()` (`src/sim/occupancy.rs:110`, `:112`) after load (`src/sim/world/mod.rs:971`, `:973`), but this slot did not prove gamemd save/load rebuild order.
- Exact global semantic name and writer taxonomy for `CellClass+0xDC` remains mixed in prior docs; the validator read contract is verified, but the full writer lifecycle is outside this slot (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:190`).
- Exact class identity for the RTTI `0x24` blocker helper in `CheckOccupancy` remains unresolved, though the blocker effect is verified (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:94`, `:189`).
- Dummy cell initial values were not dumped in the validator report (`docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:191`).

## Stale-Doc Replacement Wording Found

- `docs/research/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: replace the summary "`Checks the entire foundation rectangle for object occupation`" with "`Checks the entire rectangle for cell-field blockers, object-list blockers, optional house/site reservation bits, and playfield containment; dynamic unit occupancy is only one adjacent concept and `Cell+0xDC` is skipped when the layer argument is `-1`.`" Evidence: `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:184`.
- `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace "`Occupancy check: CellRect__CheckOccupancy (0x586780) -- no other units blocking`" with "`Optional rectangle occupancy/blocker check: `CellRect__CheckOccupancy @ 0x00586780`; it rejects `Cell+0x44/+0x4C/+0x11C`, RTTI `0x24`, building occupants, optional `Cell+0xDC` reservation bits, and out-of-playfield rectangles. In `Find_Nearby_Passable_Cell`, the reservation layer is `-1`, so `Cell+0xDC` is skipped.`" Evidence: `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:185`.
- `docs/research/CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`: replace "Current `OccupancyGrid::add` appends every occupant" with: "Current `OccupancyGrid::add` accepts `CellListInsertion` and inserts non-buildings before the first same-layer occupant while inserting buildings after the last same-layer occupant." Evidence: `docs/research/CELL_OBJECT_LIST_ORDERING_PARITY_GHIDRA_REPORT.md:116`.

## Status

COMPLETE for this slot's target: a non-editing first migration slice and acceptance-test chain. No Rust files were modified.
