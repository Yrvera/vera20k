# Building Foundation Anchor Semantics - Ghidra Research Report

**Address(es):** `0x0041BEA0`, `0x00447AC0`, `0x00441F60`, `0x005683C0`, `0x005687F0`, `0x00458A00`, `0x0073F0A0`, `0x004AC2B0`, `0x006DA5C0`, `0x00716150`, `0x0045EE70`, `0x00464AF0`, `0x00474DA0`
**Investigation Mode:** coverage-map
**Claimed Scope:** Rust-facing contract for building anchor/reference cells: stored building origin, foundation origin, AddOccupy/RemoveOccupy offset origin, `NumberImpassableRows` west/X comparison, placement/passability separation, and selection/bracket reference points where verified.
**Non-Scope:** full single-click pixel-to-object hit-test, full `UnitClass::Can_Enter_Cell` return-code matrix after a building remains a blocker, every `CellClass+0x100` visual reader, and docking/exit offsets beyond contrast examples.
**Confidence:** High for anchor/origin, placement, occupy, hidden occupancy, row-helper polarity, and bracket extents; Medium for single-click building hit coverage because this pass verified selection gates/bandbox center behavior but did not exhaust the object-picking producer.
**Active in YR:** Yes for standard building placement, occupancy, passability, selection, and bracket paths; conditional branches are called out below.

## 0. Investigation Contract

**Target question:** What is the building/foundation reference-point contract future Rust code should use for coordinate semantics: stored building position, foundation origin, AddOccupy/RemoveOccupy offsets, `NumberImpassableRows`, and selection/click reference points?

**Non-goals:** Do not implement Rust; do not patch in-repo docs; do not re-open full docking/queue/exit, tactical screen inverse, or cell-center research owned by other coordinate slots; do not claim full single-click pixel hit-test completion.

**Evidence needed to mark COMPLETE for this slot:** binary evidence for origin cell vs center coordinate; binary evidence for base foundation vs hidden occupancy separation; binary evidence for offset signedness/bounds/sentinel behavior; binary evidence for `NumberImpassableRows` origin/X polarity and stock liveness; Rust scan naming affected surfaces and proposed tests.

**Stop conditions:** stop after the Rust-facing contract is coherent and every scoped claim has address/doc/INI evidence; defer only full single-click hit coverage and full downstream return-code/hidden-visual consumers.

## 1. Overview

The core contract is: a building's stored/live position is its foundation origin cell center, not its geometric center. Systems that need the center call a different virtual (`BuildingClass::GetCoords`) which projects from the stored origin using foundation width/height.

The foundation origin is also the reference point for base foundation cell-list offsets, `AddOccupy%d`/`RemoveOccupy%d` offsets, hardcoded dock examples such as refinery `origin+(3,1)`, and the `NumberImpassableRows` helper's west-side X comparison. The binary keeps at least three concepts separate: base foundation object-list cells, hidden occupancy cells, and live per-candidate passability skips.

## 2. Class Layout / Key Offsets

| Field / slot | Meaning | Evidence | Active in YR |
|---|---:|---|---|
| `ObjectClass+0x9C/+0xA0/+0xA4` | stored coordinate; for buildings this is foundation-origin cell center | prior `0x005F6940`; `0x0041BEA0` reads X/Y | Yes |
| Building vtable `+0x1B8` | returns packed cell from stored X/Y only | assembly `0x0041BEA1..0x0041BEDA` | Yes |
| Building vtable `+0x48` | returns foundation-center coordinate | assembly `0x00447AC4..0x00447B04` | Yes |
| `BuildingTypeClass+0xEF0` | `Foundation=` enum id | parser `0x00461225..0x00461257`; helper `0x00474DA0` | Yes |
| `BuildingTypeClass+0xDFC` | base foundation cell-list pointer | assignment `0x0046152C..0x00461541`; getter `0x0045EC20` | Yes |
| `BuildingTypeClass+0xED4` | foundation exit-cell table pointer | assignment `0x00461547..0x0046156A` | Yes |
| `BuildingTypeClass+0x1620` | `NumberImpassableRows`, sentinel `-1` | helper `0x00458A20`; parser prior `0x0046013A` | Conditional |
| `BuildingTypeClass+0x1624..0x1660` | eight signed `AddOccupy1..8` `(x,y)` pairs | parse loop `0x00461425..0x00461486` | Conditional on `CanHideThings` effect |
| `BuildingTypeClass+0x1664..0x16A0` | eight signed `RemoveOccupy1..8` `(x,y)` pairs | parse loop `0x0046148A..0x004614E8` | Conditional on `CanHideThings` effect |
| `BuildingTypeClass+0x1766` | `CanHideThings`, default true | read near `0x0046140F`; hidden writers | Conditional |
| `CellClass+0xE4/+0xE8` | ground/bridge object lists scanned by `Can_Enter_Cell` | `0x0073F0A0` sibling reports | Yes |
| `CellClass+0x100` | hidden occupancy counter adjusted by height/add/remove | writers `0x005683C0`, `0x005687F0` | Conditional; not ordinary building object-list occupancy |

## 3. Core Logic

### 3.1 Stored anchor vs center

`ObjectClass__Get_Cell_Packed @ 0x0041BEA0` reads only stored X/Y at `+0x9C/+0xA0`, applies signed division by 256 (`CDQ; AND EDX,0xFF; ADD; SAR 8`), and writes packed `(x,y)` to the caller out pointer. No foundation size, `QueueingCell`, `DockingOffset`, `TargetCoordOffset`, or add/remove table participates.

`BuildingClass__GetCoords @ 0x00447AC0` computes `stored.x + foundation_width * 128 - 128`, `stored.y + foundation_height * 128 - 128`, and stored Z. For a `4x3` building at origin `(ox,oy)`, `+0x1B8` returns `(ox,oy)`, while `+0x48` returns lepton center `(ox*256+384, oy*256+256, z)`.

### 3.2 Foundation parser and base cell-list origin

`Foundation=` is a fixed 22-entry enum table, not a free-form `WxH` parser. The building parser reads art/image section first, stores that result, then reads the rules/building section using the art result as default and only overwrites on nonzero result (`0x00461225..0x00461257`). The base foundation pointer is computed as `0x0089C900 + foundation_id * 120`; the exit-cell table pointer is `0x0089D368 + foundation_id * 120`.

The table offsets are relative to the building placement/origin cell. The normal foundation list is sentinel-terminated by `(0x7FFF,0x7FFF)` per prior `Place_OccupyMap` and placement-validator reports.

### 3.3 Placement and normal occupancy use base foundation only

`FUN_00716150 @ 0x00716150` and `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` walk the base foundation cell list from vtable `+0x90`. They do not read `AddOccupy`, `RemoveOccupy`, or `CanHideThings`.

`BuildingClass::Place_OccupyMap @ 0x00441F60` calls the object's foundation-list virtual (`vtable+0x108` at `0x00441F90`) and marks base foundation cells. The entry path nearby gets the object coordinate, converts it to a cell with the signed divide-by-256 pattern, then applies listed offsets. No add/remove table is in this normal occupancy path.

### 3.4 AddOccupy/RemoveOccupy are hidden-occupancy modifiers

The parser loops exactly eight numbered keys for both families. Missing entries use `(0xFFFF,0xFFFF)` sentinel defaults. The offsets are signed short pairs and are relative to the same foundation origin as the base list. Stock examples include negative offsets such as `GAREFN AddOccupy1=-1,0` and positive in-foundation removals such as `RemoveOccupy1=3,1`.

`TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` first adds the object to base foundation content lists, then, only for building objects with `CanHideThings`, updates `CellClass+0x100` from diagonal `OccupyHeight` coverage, `AddOccupy`, and `RemoveOccupy`. `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0` reverses diagonal/add increments; `RemoveOccupy` is not a normal object-list removal.

Do not treat `RemoveOccupy=3,1` as "the building is absent from that cell." For stock refineries, the dock pad is still in the base `4x3` building object list; it becomes enterable through live `HasBib`/contact row-skip logic.

### 3.5 NumberImpassableRows uses game-west X columns from foundation origin

`FUN_00458A00 @ 0x00458A00` first checks `Look_up_building_in_cell(candidate_cell) == building`. If not, it returns false. If `BuildingType+0x1620 == -1`, it returns true. If `Bunker=yes` and `BuildingClass+0x2E4` is non-null, it returns true before row math.

The row math is `candidate_cell.x < building_origin_cell.x + NumberImpassableRows`. Evidence: `0x00458A51` reads candidate `CellClass+0x24` signed X; `0x00458A58` calls building vtable `+0x1B8`; `0x00458A64` reads origin X; `0x00458A67` reads `+0x1620`; `0x00458A6D..0x00458A72` compares with strict greater after adding origin+rows. This is X/game-west, not Y/top rows. The comparison is strict: `candidate_x >= origin_x + rows` is the first clear/skipped column when the helper's caller treats false as "skip this building."

The helper has live callsites inside `UnitClass::Can_Enter_Cell @ 0x0073F0A0`: a radio/contact branch and a UnitRepair/Bunker branch. It is not a static placement-grid transform.

### 3.6 Selection, brackets, and non-rectangular reference points

Verified selection/bracket facts in scope:

- Selected-building bracket dimensions use `BuildingTypeClass::Dimension2 @ 0x00464AF0`: `width_table[Foundation] << 8`, `height_table[Foundation] << 8`, and `Height * height_factor`. This is dimension-table rectangle geometry, not Add/Remove hidden occupancy and not bib extension.
- Band-box selection tests the visible object's screen point against the drag rectangle. Standard buildings are excluded from band-box selection except the documented `1x1 + UndeploysInto` exception. Evidence: `BANDBOX_SELECTION_GHIDRA_REPORT.md`, `0x006DA5C0`.
- The selection callback at `0x004AC2B0` gates a picked object through vtable `+0x138`, then `+0x13C`, then `+0x81`, then `+0x14C`; assembly spot-check at `0x004AC2D9`.

Not completed in this slot: the full single-click pixel-to-object producer for standard buildings. Current Rust uses foundation-cell containment in `src/app_entity_pick.rs::click_hits_foundation`; that may be a reasonable approximation, but this report does not verify whether gamemd single-click hit testing uses base foundation cells, rendered image coverage, visible-object anchor ordering, or another building-specific picker. Do not use this report as proof for single-click non-rectangular hit coverage.

## 4. INI Keys

| Key | Binary behavior | Stock liveness |
|---|---|---|
| `Foundation=` | fixed enum id; art/image first, rules/building can override only with nonzero id | Yes; widespread in `artmd.ini` |
| `AddOccupy1..8=` | signed origin-relative hidden-occupancy offsets, eight independent keys | Conditional; stock GAREFN, NAREFN, factories, tech buildings |
| `RemoveOccupy1..8=` | signed origin-relative hidden-occupancy counter cancellations, not object-list removals | Conditional; stock refineries/factories/depot/civilian buildings |
| `CanHideThings=` | gates hidden occupancy height/add/remove effects; default true | Conditional; many stock buildings true |
| `OccupyHeight=` | hidden occupancy depth input; binary uses hidden-counter path, not placement height equality | Conditional |
| `Bib=yes` | live `Can_Enter_Cell` east-neighbor relaxation; does not extend foundation list | Conditional; stock refineries/factories |
| `NumberImpassableRows=` | live helper X/west column gate from foundation origin; `-1` sentinel means no skip; `0` makes ordinary row comparison false everywhere | Conditional; stock refineries, war factories, repair depots, `NATBNK`, `CAOUTP` |
| `UndeploysInto=` | relevant to band-box 1x1 building exception and deploy/undeploy conversion; not a foundation offset | Conditional |
| `QueueingCell`, `DockingOffset%d`, `ExitCoord` | separate docking/exit reference systems; contrast only in this report | Conditional; owned by docking/queue/exit slot |

## 5. Integration Points

| System | Reference point / cells | Evidence | Rust-facing contract |
|---|---|---|---|
| Stored building position | foundation origin cell center | `0x0041BEA0`, prior `0x005F6940` | `GameEntity.position.rx/ry` for structures should remain origin |
| Building center coordinate | origin plus `(w*128-128, h*128-128)` | `0x00447AC0` | use a separate helper for center/target APIs |
| Placement validation | base foundation cell list from origin | `0x00716150`, `0x0045EE70` | do not include add/remove, bib, or row skip |
| Normal object-list occupancy | base foundation cells from origin | `0x00441F60`, `0x005683C0` | occupancy grid should hold base foundation cells |
| Hidden/behind occupancy | base + diagonal height + add/remove, gated by `CanHideThings` | `0x005683C0`, `0x005687F0` | separate hidden occupancy from movement/placement |
| Unit vehicle entry row skip | live candidate-cell object-list skip | `0x00458A00`, `0x0073F0A0` | implement at live cell-entry/object-blocker layer, not as permanent footprint erasure |
| Brackets/pips/minimap-style extents | width/height table rectangle from foundation id | `0x00464AF0`, bracket reports | do not use add/remove hidden occupancy for extents |
| Band-box selection | visible-object screen point; buildings excluded except 1x1+UndeploysInto | `0x006DA5C0`, `BANDBOX_SELECTION...` | do not infer drag-select from footprint coverage |
| Single-click selection | touched, not exhausted | `0x004AC2B0` gate only | follow-up needed before changing non-rect click behavior |

## 6. Current Rust Implementation Status

| Rust surface | Status | Evidence / delta |
|---|---|---|
| `src/rules/foundation.rs` | matches fixed parser table for dimensions | includes `3x3Refinery` and `0x0`; case-insensitive helper |
| `src/sim/production/production_tech.rs::building_base_foundation_cells` | mostly aligned for rectangular stock foundations | returns rectangle from origin; binary walks table cell-list, so keep an eye on special foundation ids and exit lists |
| `building_hidden_occupancy_cells` | conceptually aligned as hidden set, but simplified | applies base/add/remove as a set; binary also applies diagonal `OccupyHeight` counter details and `CanHideThings` gate elsewhere |
| `building_footprint_cells` | naming hazard | compatibility alias returns hidden occupancy, not real foundation; no production callers found except tests, but future code should avoid it for placement/passability/selection |
| `world_spawn.rs` structure occupancy | aligned with base foundation | map and runtime structure spawn insert `building_base_foundation_cells` |
| `bump_crush.rs` static blockers | mostly aligned for base+bib; row inactive | calls `building_movement_blocking_cells_for_state(... number_rows_active=false)` |
| `movement_occupancy.rs::build_live_vehicle_building_entry_skip_map` | has live row-skip layer | builds skip map from base foundation and `decide_live_vehicle_building_entry`; verify candidate lookup semantics if changed |
| `cell_entry.rs::decide_live_vehicle_building_entry` | matches row-helper polarity | `candidate_x >= origin_x + rows` => skip; uses `-1` and bunker occupied fast-keep |
| `app_entity_pick.rs::click_hits_foundation` | unverified vs gamemd single-click | uses foundation dimension rectangle; this report does not prove the binary click producer does that |
| `production_placement.rs` | base dimensions; ensure no same-height or hidden occupancy contamination | current scan only, not re-tested here |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Initial target/non-goals/evidence/stop conditions | verified | section 0 | none |
| Stored origin and `+0x1B8` packed cell | verified | `0x0041BEA0`, prior `BUILDINGCLASS_GETCELLLOCATION...` | none |
| Center coordinate virtual | verified | `0x00447AC0` assembly | none |
| Foundation parser table/read order | verified | `0x00474DA0`, `0x00461225..0x00461257` | upstream MIX merge out of scope |
| Base foundation pointer/list assignment | verified | `0x0046152C..0x0046156A`, `0x0045EC20` | exact table memory contents not re-dumped here |
| Placement validators | verified by prior + spot-check | `0x00716150`, `0x0045EE70` reports | exact flag names for `Cell+0x140` non-scope |
| Normal occupancy path | verified by prior + spot-check | `0x00441F60`, `0x005683C0` | exact cell owner side effects not expanded |
| Add/Remove parser and hidden writers | verified | `0x00461425..0x004614E8`, `0x005683C0`, `0x005687F0` | full `Cell+0x100` render readers non-scope |
| `NumberImpassableRows` helper | verified | `0x00458A00`, assembly `0x00458A51..0x00458A72` | full post-keep return-code matrix non-scope |
| Stock liveness for row keys | verified by existing docs/INI | `rulesmd.ini` stock sections and row reports | none for listed structures |
| Bracket dimensions | verified by existing docs + spot-check | `0x00464AF0`, `FOUNDATION_PARSER...` | exact final pixels non-scope |
| Band-box selection reference point | verified from prior doc | `0x006DA5C0`, `BANDBOX_SELECTION...` | none for band-box |
| Single-click building hit producer | touched-not-exhausted | `0x004AC2B0` gate spot-check | trace object producer/picking geometry |
| Rust surface audit | verified | Codegraph and `rg` scan on 2026-05-22 | tests after implementation changes |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Is this a coverage-map or exhaustive-slice? -> Coverage-map with a verified Rust-facing contract; single-click hit geometry is explicitly deferred.` (evidence: section 7)
- `[RESOLVED] OQ-002 - What is the building stored anchor? -> Foundation-origin cell center in `ObjectClass+0x9C/+0xA0/+0xA4`.` (evidence: `0x0041BEA0`, prior `0x005F6940`)
- `[RESOLVED] OQ-003 - What does BuildingClass vtable `+0x1B8` return? -> Origin cell from stored X/Y, signed-divided by 256.` (evidence: `0x0041BEA1..0x0041BEDA`)
- `[RESOLVED] OQ-004 - What returns the center coordinate? -> `BuildingClass::GetCoords @ 0x00447AC0`, origin plus `w*128-128`, `h*128-128`.` (evidence: `0x00447AC4..0x00447B04`)
- `[RESOLVED] OQ-005 - Is `Foundation=` free-form? -> No, fixed enum table through `0x00474DA0`.` (evidence: `FOUNDATION_PARSER...`, `0x00474DA0`)
- `[RESOLVED] OQ-006 - Which origin do foundation offsets use? -> The building placement/foundation origin cell.` (evidence: `0x00441F60`, `0x00716150`, `0x0045EE70`)
- `[RESOLVED] OQ-007 - Do AddOccupy/RemoveOccupy alter base foundation object lists? -> No.` (evidence: `0x00441F60`, `0x005683C0`, `0x005687F0`)
- `[RESOLVED] OQ-008 - Are Add/Remove offsets signed and origin-relative? -> Yes, parsed into signed pairs with negative stock examples and applied as origin deltas in hidden occupancy writers.` (evidence: `0x00461425..0x004614E8`, `artmd.ini` stock examples)
- `[RESOLVED] OQ-009 - How many Add/Remove keys are parsed? -> Eight numbered keys each, sentinel `(0xFFFF,0xFFFF)` for absent/malformed.` (evidence: `0x00461425..0x004614E8`, prior report)
- `[RESOLVED] OQ-010 - What does `CanHideThings` gate? -> Hidden occupancy height/add/remove effects, not base placement or object-list occupancy.` (evidence: `0x0046140F`, `0x005683C0`, `0x005687F0`)
- `[RESOLVED] OQ-011 - Does `NumberImpassableRows` count Y rows/top rows? -> No, it compares candidate X against origin X plus rows.` (evidence: `0x00458A51..0x00458A72`)
- `[RESOLVED] OQ-012 - Is the row comparison inclusive? -> First clear column is `candidate_x >= origin_x + rows`; helper keep/true is strict `<`.` (evidence: `0x00458A6D..0x00458A72`)
- `[RESOLVED] OQ-013 - Is row behavior static placement data? -> No, it is a live `Can_Enter_Cell` object-list skip/keep decision.` (evidence: `0x0073F0A0` row callsites)
- `[RESOLVED] OQ-014 - What should brackets use? -> Dimension table rectangle from foundation id and `Height`, not add/remove hidden occupancy.` (evidence: `0x00464AF0`)
- `[RESOLVED] OQ-015 - What does band-box selection test? -> visible object screen point in rectangle; buildings mostly excluded.` (evidence: `BANDBOX_SELECTION_GHIDRA_REPORT.md`, `0x006DA5C0`)
- `[RESOLVED] OQ-016 - Does current Rust keep base occupancy separate? -> Mostly yes in spawn/static movement paths; `building_footprint_cells` remains a naming hazard alias to hidden occupancy.` (evidence: `src/sim/production/production_tech.rs`, `world_spawn.rs`, `bump_crush.rs`)
- `[RESOLVED] OQ-017 - Does current Rust row helper use west/X polarity? -> Yes in `decide_live_vehicle_building_entry`.` (evidence: `src/sim/pathfinding/cell_entry.rs`)
- `[DEFERRED] OQ-018 - Exact single-click building pixel/object producer for non-rectangular foundations.` (category: `requires-different-system-context`; reason: this slot verified selection callback gates but not the producer that selects the object under the mouse; next-step-if-pursued: trace left-click object lookup before `0x004AC2B0` and compare to rendered SHP/foundation/base-list cells)
- `[DEFERRED] OQ-019 - Full `CellClass+0x100` downstream visual consumers.` (category: `out-of-scope`; reason: separate hidden occupancy/behind-object reports own this; next-step-if-pursued: xref `CellClass+0x100` readers)
- `[DEFERRED] OQ-020 - Final `Can_Enter_Cell` return code after row helper keeps a building blocker.` (category: `out-of-scope`; reason: this contract only needs skip-vs-keep and origin polarity; next-step-if-pursued: full building blocker return-code matrix)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Structure `position.rx/ry` is foundation origin, while center coordinate is a separate projection | `0x0041BEA0`, `0x00447AC0` | mostly none; center helper may be ad hoc in consumers | `src/util/lepton.rs`, `src/sim/world/world_spawn.rs`, combat/docking helpers | Keep origin and center APIs distinct | `building_origin_cell_and_center_coord_are_distinct_for_garefn_4x3` | Do not "fix" structure position to center |
| Placement and normal occupancy use base foundation cells from origin | `0x00716150`, `0x0045EE70`, `0x00441F60` | currently mostly aligned | `src/sim/world/world_spawn.rs`, `src/sim/production/production_placement.rs`, `src/sim/production/production_tech.rs` | Use `building_base_foundation_cells` for placement/occupancy | `garefn_spawn_registers_all_4x3_base_cells_in_occupancy` | Do not use `building_footprint_cells` alias for real occupancy |
| Add/Remove are hidden-occupancy modifiers, not normal object-list footprint | `0x00461425..0x004614E8`, `0x005683C0`, `0x005687F0` | concept present; exact `OccupyHeight`/counter model simplified | `building_hidden_occupancy_cells`, hidden/behind render path | Keep hidden occupancy separate and gated by `CanHideThings` | `garefn_remove_occupy_does_not_remove_dock_pad_from_base_foundation` | Do not make `RemoveOccupy` path-unblock by deleting base cell |
| `NumberImpassableRows` is a live X/west skip with strict `<` keep polarity | `0x00458A00`, `0x0073F0A0` | row decision exists; keep live candidate semantics | `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs` | Apply only after contact or UnitRepair/Bunker branch reaches the helper | `contacted_refinery_rows3_skips_x3_but_keeps_x2` | Do not bake row count into static `PathGrid` |
| `Bib=yes` relaxes east edge, not a bib row extension | `BIB_ADJACENT...`, `0x0073F7D3` sibling reports | static helper approximates east-edge base topology | `building_movement_blocking_cells_for_state`, path grid blocker build | Use base foundation topology for east-neighbor test | `bib_garefn_uses_base_4x3_east_edge_not_add_remove_shape` | Do not apply bib to hidden-occupancy set |
| Bracket/dimension extents use foundation width/height table | `0x00464AF0`, `FOUNDATION_PARSER...` | Rust foundation table now matches; selection brackets should consume it | `src/rules/foundation.rs`, `src/app_selection_brackets.rs`, render instance generation | Keep bracket extents dimension-table based | `foundation_3x3refinery_dimensions_are_3x3_for_brackets` | Do not parse arbitrary `WxH` strings for parity |
| Band-box uses visible-object point, not footprint intersection | `BANDBOX_SELECTION...`, `0x006DA5C0` | Rust excludes all structures; misses 1x1+UndeploysInto exception | `src/app_entity_pick.rs`, `src/sim/selection.rs` | Preserve point-in-rect drag selection and add exception if needed | `bandbox_excludes_garefn_but_can_include_1x1_undeploysinto_building` | Do not use foundation cell coverage for band-box |
| Single-click building hit geometry is not proven by this report | `0x004AC2B0` only gates after object picked | Rust uses dimension rectangle containment | `src/app_entity_pick.rs::click_hits_foundation` | Leave or investigate before parity-sensitive changes | `single_click_nonrect_foundation_hit_test_requires_binary_trace` | Do not cite this report as proof for single-click non-rect hits |

### Negative Facts / Do Not Do

- Do not call the stored building position a "center"; it is the origin cell center. Use `+0x48` semantics when a center is needed.
- Do not apply `AddOccupy` or `RemoveOccupy` to placement validation, normal occupancy grid, C4/selection geometry, or movement blockers unless a specific binary consumer proves it.
- Do not interpret `NumberImpassableRows` as Y rows, top rows, or a rectangle shrink from the north. It is X/game-west.
- Do not make `NumberImpassableRows=0` mean "all cells impassable"; in the helper, ordinary row math returns false everywhere, so the caller may skip the building everywhere unless a fast-true branch applies.
- Do not treat `QueueingCell`, `DockingOffset`, or `ExitCoord` as alternate definitions of the building anchor.
- Do not use `building_footprint_cells` as a generic name in new Rust; it returns hidden occupancy, not the real foundation.

### Remaining Uncertainty

- Full single-click building hit-test geometry remains unverified. This is the main remaining gap for "non-rectangular foundation" click behavior.
- Exact `CellClass+0x100` hidden-occupancy visual readers are delegated to hidden/behind-object research.
- Exact final `Can_Enter_Cell` return codes after helper true are delegated to a full building-blocker matrix.
- The binary base foundation cell-list table contents were not re-dumped here; existing reports verify the pointer/list mechanism, while current Rust uses rectangle dimensions for base foundations.

### Proposed Rust Tests

- `building_origin_cell_and_center_coord_are_distinct_for_garefn_4x3`
- `garefn_spawn_registers_all_4x3_base_cells_in_occupancy`
- `garefn_remove_occupy_does_not_remove_dock_pad_from_base_foundation`
- `hidden_occupy_offsets_are_origin_relative_and_do_not_affect_base_cells`
- `contacted_refinery_rows3_skips_x3_but_keeps_x2`
- `unitrepair_rows1_skips_depot_east_columns_but_keeps_west_column`
- `empty_natbnk_rows0_skips_all_foundation_cells`
- `occupied_natbnk_rows0_keeps_foundation_cells`
- `bib_garefn_uses_base_4x3_east_edge_not_add_remove_shape`
- `bandbox_excludes_garefn_but_can_include_1x1_undeploysinto_building`
- `single_click_nonrect_foundation_hit_test_requires_binary_trace`

## Sources

- Ghidra read-only assembly/context spot-checks: `0x0041BEA0`, `0x00447AC0`, `0x00441F60`, `0x005683C0`, `0x005687F0`, `0x00458A00`, `0x00461225`, `0x00461425`, `0x0046148A`, `0x0046152C`, `0x004AC2B0`, `0x006DA5C0`.
- Existing research reports: `BUILDING_POSITION_FOUNDATION_ORIGIN_PARITY_GHIDRA_REPORT.md`, `BUILDINGCLASS_GETCELLLOCATION_VTABLE_0X1B8_ANCHOR_GHIDRA_REPORT.md`, `BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`, `BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`, `BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`, `REFINERY_DOCK_PAD_CAN_ENTER_CELL_STACK_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`, `UNITREPAIR_BUNKER_NUMBER_IMPASSABLE_ROWS_SECOND_CALLSITE_GHIDRA_REPORT.md`, `BIB_ADJACENT_CELL_DIRECTION_SOURCE_GHIDRA_REPORT.md`, `BANDBOX_SELECTION_GHIDRA_REPORT.md`, `SELECTION_GATES_GHIDRA_REPORT.md`, `building-selection-brackets/FOUNDATION_PARSER_TABLE_BRACKET_EXTENTS_GHIDRA_REPORT.md`, `building-selection-brackets/BUILDING_BRACKET_DEPTH_DOMINANT_RASTER_REACHABILITY_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan: `src/rules/foundation.rs`, `src/sim/production/production_tech.rs`, `src/sim/world/world_spawn.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/movement/bump_crush.rs`, `src/app_entity_pick.rs`, `src/app_selection_brackets.rs`.
