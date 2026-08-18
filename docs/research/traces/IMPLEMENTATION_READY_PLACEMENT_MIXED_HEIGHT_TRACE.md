# Implementation Trace - Ready GAPOWR Placement Over Mixed Terrain Heights

Scope: one concrete scenario only. A ready Allied `GAPOWR` is placed on a clear, in-build-area 2x2 land foundation with mixed terrain heights:

- Placement origin: `(15,10)`
- Foundation cells: `(15,10)=0`, `(16,10)=1`, `(15,11)=2`, `(16,11)=3`
- Existing owned build-area provider: normal `GACNST` at `(10,10)`, clear terrain, no overlay blockers, no structure overlap, no bridge deck cells
- `GAPOWR` data: `rulesmd.ini` `[GAPOWR] Adjacent=2`, `Power=200`; `artmd.ini` `[GAPOWR] Foundation=2x2`, `Buildup=GAPOWRMK`, `Height=4`

No Rust files, INI files, or in-repo docs were modified. I did not run `cargo test` because the task allowed exactly one written file and a test run would write build artifacts outside this report.

## Pipeline

Ready placement preview -> click schedules `PlaceReadyBuilding` -> sim command validates ready item, per-cell terrain/overlap, and build area -> `spawn_object` creates the structure at origin height -> ready item is consumed -> renderer shows buildup at the spawned entity screen anchor.

## Entry Points Covered

- Rust preview: `src/sim/production/production_placement.rs:22`
- Rust click scheduling: `src/app_commands.rs:248`
- Rust sim command application: `src/sim/world/world_commands.rs:600`
- Rust placement implementation: `src/sim/production/production_placement.rs:176`
- Rust production spawn path: `src/sim/world/world_spawn.rs:278`
- Rust SHP/build-up draw path: `src/app_instances/shp.rs:104`
- gamemd ready placement: `HouseClass::Place_Production @ 0x004FB0E0`
- gamemd commit placement: `BuildingClass::Unlimbo @ 0x00440580`
- gamemd placement validator for comparison: `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70`

## Stage Trace

### Stage 1 - Data and Foundation Size

Rust reads `GAPOWR` as a normal land building with foundation `2x2`, so the concrete footprint has exactly 4 cells. `foundation_dimensions` drives both preview and placement validation.

gamemd `BuildingTypeClass::GetFoundationWidth @ 0x0045EC90` returns `g_FoundationWidthTable[Type+0xEF0]`; `GetFoundationHeight @ 0x0045ECA0` returns `g_FoundationHeightTable[Type+0xEF0]` when called with the normal `0` argument. Retail `artmd.ini` maps `GAPOWR` to `2x2`.

Output: Rust width `2`, height `2`; gamemd width `2`, height `2`.

Verdict: `PASS`

### Stage 2 - Preview Validation

Rust `placement_preview_for_owner` calls `evaluate_building_placement`, then computes 4 per-cell booleans through `cell_placeable`. With the concrete clear cells and no overlap, all 4 cell booleans are `true`; because build area is true, preview `valid=true`.

gamemd cursor-preview validation is not the same function as the commit path, and I did not compute the exact cursor-preview output for this same screen/cell input.

Output: Rust preview `valid=true`, `cell_valid=[true,true,true,true]`; gamemd preview output not numerically computed.

Verdict: `UNCHECKED`

### Stage 3 - Height Equality Gate

Rust current implementation has no same-height gate in ready placement. `evaluate_building_placement` receives `_height_map` at `src/sim/production/production_placement.rs:275`, but the underscore parameter is unused. The only 2x2 footprint loop at `src/sim/production/production_placement.rs:296` calls `cell_placeable`; it does not compare `(16,10)=1`, `(15,11)=2`, or `(16,11)=3` against `(15,10)=0`.

gamemd `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` walks foundation offsets and checks in-bounds, overlay/building cell state, first object, upgrade/allied overlap, and scatter side effects. The decompiled active YR function contains no terrain-height read and no equality comparison against a reference cell. `BuildingClass::Unlimbo @ 0x00440580` likewise contains no all-foundation-cells-same-height reject before normal placement.

Output: Rust same-height rejects `0`; gamemd same-height rejects `0`.

Verdict: `PASS`

### Stage 4 - Commit Validation Result

Rust `place_ready_building` verifies the ready queue contains `GAPOWR`, then calls `evaluate_building_placement`. For this concrete clear mixed-height footprint, the validation path finds no overlap, no terrain block, and in-build-area true, so it returns `Ok(())`; `place_ready_building` proceeds past validation.

gamemd ready placement path `HouseClass::Place_Production @ 0x004FB0E0` constructs coordinates from the selected cell as `cell_x * 256 + 128`, `cell_y * 256 + 128`, `z=0`, then calls the object's `vtable+0xD8`, which is `BuildingClass::Unlimbo @ 0x00440580`. The inspected active YR commit path has no mixed-height reject. I did not compute every non-height gamemd blocker state for a live retail map instance, but the scenario explicitly states clear legal cells.

Output: Rust commit validation accepts; gamemd inspected commit path has no height-only reject and reaches `Unlimbo` for legal cells.

Verdict: `PASS`

### Stage 5 - Spawn and Stored Position

Rust `spawn_object` reads only the origin height: `height_map[(15,10)] = 0`, then calls `spawn_object_at_height`. `GameEntity::new` stores `rx=15`, `ry=10`, `z=0`, and computes screen anchor with `lepton_to_screen`: `screen_x=(15-10)*30=150`, `screen_y=(15+10)*15+15-0*15=390`.

gamemd `HouseClass::Place_Production` passes coord `(3968,2688,0)` for cell `(15,10)` into `BuildingClass::Unlimbo`. I did not compute the resulting stored building coord, final draw anchor, and screen pixel for this exact `GAPOWR` placement through gamemd's object coordinate/render path.

Output: Rust entity anchor `(rx=15, ry=10, z=0, sx=150, sy=390)`; gamemd final anchor not numerically computed.

Verdict: `UNCHECKED`

### Stage 6 - Ready Queue Mutation

Rust only removes the ready item after successful spawn. For a one-item ready queue `[GAPOWR]`, `ready_queue.remove(index)` leaves length `0`, then the owner entry is removed from `ready_by_owner`.

gamemd success path in `HouseClass::Place_Production @ 0x004FB0E0` calls `FactoryClass::CompletedProduction` after successful `Unlimbo`. I did not compute the exact factory/sidebar ready-state fields before and after this concrete placement.

Output: Rust ready count `1 -> 0`; gamemd exact ready/sidebar numeric fields not computed.

Verdict: `UNCHECKED`

### Stage 7 - Visible Placement Result

Rust tags the new structure with `BuildingUp { elapsed_ticks: 0, total_ticks: 30 }` at `src/sim/production/production_placement.rs:239`. The SHP render path sees `building_up.is_some()` and uses `GAPOWR_MAKE`/make-frame selection when available. The player should see a placed power plant build-up at the spawned anchor instead of a red/rejected placement.

gamemd successful `HouseClass::Place_Production` calls `FactoryClass::CompletedProduction`, plays the placement sound for the player, clears placement UI state, and the placed building renders through its standard Unlimbo/building visual path. I did not compute the exact first visible make frame, sound id, UI mode fields, or pixel anchor for this concrete placement.

Output: Rust visible outcome is placed/building-up; gamemd visible outcome is placed/building-up for legal cells, but exact frame/pixel/audio values were not computed.

Verdict: `UNCHECKED`

## Failures

None found for the implemented mixed-height ready-placement fix. The old player-visible bug, where mixed terrain height alone made a clear GAPOWR placement invalid, is not present in the current Rust ready-placement validation path.

## Not Implemented

None found in this concrete mechanic. Exact sidebar/audio/render-frame parity remains unchecked, not classified as missing.

## Adjacent Findings

- The Rust ready-placement preview and commit share validation more tightly than gamemd's cursor-preview and `Place_Production`/`Unlimbo` commit path. I did not trace preview algorithm parity here because this run is limited to the mixed-height implementation fix.
- Rust uses origin-cell height for the spawned building. Whether gamemd's visible anchor for a legal mixed-height foundation also resolves purely from the clicked/origin cell needs a separate coordinate/render trace with exact pixels.

## Verdict Tally

PASS: 3 | FAIL: 0 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
