# Building Footprint Consumer Discrepancy Audit - Ghidra Research Report

**Address(es):** `0x00441F60`, `0x005683C0`, `0x005687F0`, `0x00519630`, `0x00456580`, `0x006565A0`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** current Rust consumers of `building_footprint_cells`, `building_movement_blocking_cells`, and nearby base-foundation consumers, classified against known/spot-checked `gamemd.exe` behavior.  
**Non-Scope:** implementing fixes, exhaustive passability-chain reverse engineering, exhaustive tactical selection internals, and full placement-validator decompilation.  
**Confidence:** High for Rust consumer inventory and for base-vs-hidden binary separation; Medium for exact passability-side bib consequences; Low/Unknown where noted.  
**Active in YR:** Yes for building placement, radar/minimap registration, C4 infantry, pathfinding, and stock buildings with `AddOccupy`/`RemoveOccupy`; hidden occupancy effects are conditional on `CanHideThings`, which defaults true and is true on stock refineries.

## 1. Executive Summary

Rust currently collapses three `gamemd.exe` concepts into one adjusted footprint:

1. base foundation cell list, used for normal building object/content/ownership/radar geometry;
2. hidden occupancy counter cells, adjusted by `OccupyHeight` + `AddOccupy` - `RemoveOccupy` and gated by `CanHideThings`;
3. per-cell unit blocking behavior, which still starts from object/content-list membership and then applies consumer-specific logic such as bib relaxation.

The direct adjusted-footprint blast radius is small but important: path-grid blocking, movement block-set construction, structure spawn occupancy registration, and C4 target-enter cells. Base-foundation consumers such as placement preview rectangles, selection brackets, target-center/range math, sell/garrison ejection, smudges, and minimap sizing mostly do not consume the wrong adjusted helper.

## 2. Binary Baseline Facts

| Behavior | Binary evidence | Active in YR |
|---|---|---|
| Building placement/occupy map walks the vtable foundation list and marks only those base cells; no add/remove offsets are read. | `BuildingClass__Place_OccupyMap @ 0x00441F60`; walks vtable `+0x108` list until `(0x7FFF,0x7FFF)`, marks overlay `0xEF`, recalcs attributes, origin cell gets type pointer. | Yes; live building placement path. |
| `AddOccupy`/`RemoveOccupy` adjust `CellClass+0x100`, not the normal foundation cell content list. | `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` increments height/add counters and decrements remove counters; `ExitCell @ 0x005687F0` reverses height/add. | Conditional; standard buildings enter this path, gated by `CanHideThings`. |
| `CanHideThings` gates hidden occupancy, and stock YR refineries set/retain it true. | Prior report constructor/read evidence, plus `ini/artmd.ini` `[NAREFN]`/`[GAREFN]` with `CanHideThings=true/True`. | Conditional, default true; active for stock refineries. |
| C4 plant claim requires the infantry's current cell to resolve to the nav-target building. | `InfantryClass__PerCellProcess @ 0x00519630`, mission `0x11`, `Look_up_building_in_cell() == param_1[0x169]` before setting building `+0x6DF`. | Yes; stock SEAL/Tanya/Psi-Corp C4 infantry path. |
| Radar building registration iterates foundation bucket offsets, not adjusted hidden occupancy offsets. | `BuildingClass__RegisterOnRadar @ 0x00456580` gets bucket from building type and calls `RadarClass__AddObjectToTracker` for every bucket cell. | Yes; building radar/minimap registration path. |
| Radar dirty marking is foundation-rect based. | `RadarClass__MarkObjectDirty @ 0x006565A0`; matches `RADAR_MINIMAP_RENDERING.md` section 15. | Yes. |

## 3. Current Rust Direct Adjusted-Footprint Consumers

| Consumer | Rust evidence | Current Rust footprint | Expected `gamemd.exe` class | Discrepancy | Severity / player-visible reason | Active in YR |
|---|---|---|---|---|---|---|
| Root helper `building_footprint_cells` | `src/sim/production/production_tech.rs:576..613` | Base rectangle + `AddOccupy` - `RemoveOccupy` | No single equivalent; base foundation and hidden occupancy are separate. | Yes, root abstraction is wrong for most consumers. | Severe/Frequent: every downstream adjusted consumer can miss real foundation cells such as GAREFN `(rx+3,ry+1)` or add non-foundation hidden cells. | Yes; stock buildings use both base foundations and hidden occupancy modifiers. |
| Bib filter helper `building_movement_blocking_cells` | `src/sim/production/production_tech.rs:616..650` | Drops east-edge cells from already-adjusted footprint when `Bib=yes`. | Consumer-specific passability starts from actual object/content cell membership; bib relaxation is not a replacement foundation. | Yes/Partial. Static filtered cells approximate one passability effect but lose object membership and compound the adjusted-footprint error. | Severe/Frequent for pathing around `Bib=yes` refineries and bases. | Yes; `Bib=yes` is stock building behavior, exact branch not exhaustively re-traced in this slot. |
| PathGrid initial building blockers | `src/app_init.rs:689..705`, `src/sim/pathfinding/core.rs:1468..1488` | Adjusted footprint, then bib-filtered. | Base placement/content list separate from hidden counter; path consumers should not treat add/remove as normal occupied foundation cells. | Yes. | Severe/Frequent: all map-load structures seed walkability; refineries are common, so harvester/unit routes can be wrong immediately. | Yes; map structures and pathfinding are standard YR. |
| PathGrid per-tick rebuild blockers | `src/app_sim_tick.rs:809..815`, `src/sim/pathfinding/core.rs:1468..1488` | Adjusted footprint, then bib-filtered. | Same as above. | Yes. | Severe/Frequent: every tick rebuild preserves the wrong blockers after placement/destruction. | Yes. |
| Movement block-set construction | `src/sim/movement/bump_crush.rs:114..153` | Adjusted footprint, then bib-filtered, inserted into ground hard-block set. | `gamemd` cell object lists contain base foundation cells; hidden counter is separate. | Yes. | Severe/Frequent: A* and cooperative block maps can route through real building cells or avoid non-foundation hidden cells. | Yes; normal ground movement. |
| Map entity spawn occupancy | `src/sim/world/world_spawn.rs:240..260` | Structure occupancy grid gets adjusted footprint. | Base foundation cells should be normal object/content occupancy; add/remove belongs to hidden counter only. | Yes. | Severe/Frequent on preplaced structures; affects targeting, C4 cell lookup, spawn exclusion, and occupancy queries. | Yes; map preplaced buildings are standard. |
| Runtime `spawn_object` structure occupancy | `src/sim/world/world_spawn.rs:429..439` | Structure occupancy grid gets adjusted footprint. | Same base-foundation-only normal occupancy. | Yes. | Severe/Frequent for every produced/deployed building after game start. | Yes. |
| C4 target footprint / enter-cell selection | `src/sim/world/world_orders.rs:426..449`, `572..581`, `602..622` | Infantry can claim only inside adjusted footprint and chooses nearest adjusted cell for one-cell enter movement. | Mission `0x11` claims when current cell's building lookup equals nav target; lookup is driven by normal building cell/content ownership, not hidden add/remove cells. | Yes. | Moderate: only C4-capable infantry vs buildings with add/remove modifiers, but visible as planting from/entering the wrong cell or failing on a removed-but-real foundation cell. | Yes; C4 infantry and building targets are stock YR. |
| Helper tests | `src/sim/production/production_tech.rs:717..792`, `src/sim/pathfinding/core_tests.rs:854..867`, `src/sim/movement/movement_tests.rs:1207..1339` | Tests encode adjusted footprint and bib-filter expectations. | Tests are not gameplay, but they lock in the wrong abstraction. | Yes. | Low direct player impact; high maintenance impact because tests will resist parity fixes. | Active in Rust only; no YR runtime effect. |

## 4. Related Base-Foundation Consumers

| Consumer group | Rust evidence | Classification | Severity / reasoning | Active in YR |
|---|---|---|---|---|
| Placement preview/evaluation footprint rectangle | `src/sim/production/production_placement.rs:32..45`, `280..316`, `404..425` | Correct class for add/remove: base foundation dimensions. Indirectly polluted when `path_grid.is_walkable` was built from adjusted blockers. | Moderate: false terrain/overlap rejections can occur near AddOccupy cells or RemoveOccupy cells, but the rectangle loop itself is the right class. | Yes; building placement active. |
| Build-area adjacency | `src/sim/production/production_placement.rs:443..482` | Correct class: base provider/placed rectangles. | Low: no adjusted-footprint discrepancy found. | Yes. |
| MCV deploy/undeploy center helpers | `src/sim/world/world_spawn.rs:533..555`, `680..708` | Base dimensions, not adjusted. Separate origin/loop issues may exist, but no add/remove footprint consumer found. | Low for this audit. | Yes. |
| Selection click foundation hit | `src/app_entity_pick.rs:347..367` | Base rectangle. | Low: matches the "foundation footprint, not hidden occupancy" class; exact tactical hit rules were not exhausted. | Yes. |
| Selection brackets/health geometry | `src/app_selection_brackets.rs:197..203`, related bracket reports | Base dimensions. | Low: no adjusted-footprint discrepancy; exact bracket raster/anchor issues are separate. | Yes. |
| UI overlays/range visuals | `src/app_ui_overlays.rs:98..103`, `811..814` | Base dimensions. | Low/Unknown: no adjusted helper use; exact overlay geometry not audited here. | Yes. |
| Combat target coords/range bonus | `src/sim/combat/mod.rs:315..322`, `src/sim/combat/in_range.rs:107..112`, `300..318` | Base dimensions. | Low: target center/range bonus should be foundation-derived, not hidden-occupancy-derived. | Yes; combat targeting active. |
| Garrison and death/smudge foundation surfaces | `src/sim/combat/mod.rs:888..940`, `1218..1230`, `1426..1438`; `src/sim/production/production_sell.rs:155..275`, `384..414` | Base dimensions. | Low for add/remove footprint; separate garrison/ejection parity remains outside this audit. | Yes where feature paths are used. |
| Minimap building size | `src/render/minimap.rs:311..329`, `src/render/minimap_helpers.rs:322..325` | Base dimensions, but rendered as simplified square dot size. | Low/Unknown: no adjusted-footprint bug, but not exact radar foundation brush parity. | Yes; radar/minimap active. |
| `OccupancyGrid::rebuild` | `src/sim/occupancy.rs:108..130` | Does not expand structure footprint at all despite comment. | Unknown/Low for this audit: not an adjusted-footprint consumer, but save/load/debug rebuild can diverge from both Rust live spawn and `gamemd`. | Active in Rust; no direct YR equivalent classification beyond normal base occupancy being active. |

## 5. Sizing

Direct gameplay code consuming the wrong adjusted footprint is concentrated in four production areas:

- 2 path-grid call sites: map initialization and per-tick rebuild.
- 1 movement block-set builder used by A*/movement commands.
- 2 structure occupancy registration paths: map spawn and runtime spawn.
- 1 C4 target footprint path with adjacency and direct-enter movement.

The broad foundation-dimension surface is larger, but most of it is not contaminated by `AddOccupy`/`RemoveOccupy`; it already uses base dimensions, which is the correct class for selection geometry, target center/range, minimap sizing, placement rectangles, sell/garrison ejection, and smudge foundation requests.

## 6. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does `gamemd` merge AddOccupy/RemoveOccupy into normal building foundation cells? -> No.` Evidence: prior report plus `0x00441F60`, `0x005683C0`, `0x005687F0`.
- `[RESOLVED] OQ-2 - Which Rust sites call `building_footprint_cells` in production code? -> path grid, movement block sets, spawn occupancy, C4 target footprint.` Evidence: `rg` over `src/` and listed line references.
- `[RESOLVED] OQ-3 - Do selection/minimap/combat target geometry consume the adjusted helper? -> No; they use `foundation_dimensions`/base rectangles.` Evidence: listed Rust line references.
- `[RESOLVED] OQ-4 - Is C4 cell claim tied to building cell lookup in the binary? -> Yes, mission `0x11` checks `Look_up_building_in_cell() == NavTarget`.` Evidence: `0x00519630`.
- `[DEFERRED] OQ-5 - Exact passability reader semantics for `CellClass+0x100` and bib relaxation.` Category: requires-different-system-context. Reason: this audit sized consumers; slot 4 owns deeper path-blocking passability. Next step: trace `UnitClass::Can_Enter_Cell` and `Cell+0x100` readers.
- `[DEFERRED] OQ-6 - Exact tactical click-selection lookup for non-rectangular foundation table entries.` Category: requires-different-system-context. Reason: no direct adjusted helper use found; selection-specific reports cover bracket visuals, not complete hit testing.

## Sources

- Prior context: `docs/research/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`
- Ghidra decompiled: `0x00441F60`, `0x005683C0`, `0x005687F0`, `0x00519630`, `0x00456580`, `0x006565A0`
- Related docs: `RADAR_MINIMAP_RENDERING.md`, `RADAR_SYSTEM_COMPREHENSIVE.md`, `c4-on-bridge-repair-hut.md`
- INI checked: `ini/art.ini`, `ini/artmd.ini`, including `[GAREFN]`, `[NAREFN]`, `CanHideThings`, `AddOccupy`, `RemoveOccupy`
- Rust scanned: `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/world/world_spawn.rs`, `src/sim/world/world_orders.rs`, `src/sim/production/production_placement.rs`, `src/sim/occupancy.rs`, `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, `src/app_entity_pick.rs`, `src/app_selection_brackets.rs`, `src/app_ui_overlays.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/in_range.rs`, `src/sim/production/production_sell.rs`
