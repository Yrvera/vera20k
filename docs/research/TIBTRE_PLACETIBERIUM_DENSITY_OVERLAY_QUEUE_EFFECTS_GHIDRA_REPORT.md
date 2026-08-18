# TIBTRE PlaceTiberium Density Overlay Queue Effects - Ghidra Research Report

**Address(es):** `0x00483780`, `0x004838E0`, `0x00487190`, `0x007235A0`, `0x00722AF0`, `0x006551C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `CellClass::PlaceTiberium` effects as reached from `TerrainClass::AI -> CellClass::SpreadTiberium(force=1)` for TIBTRE spawning: density/data, empty-vs-existing target behavior, overlay type selection, dirty effects, and queue insertion.
**Non-Scope:** TIBTRE animation/RNG timing before `SpreadTiberium`, exact direction-offset cardinal labels, full `CanPlaceTiberium` rejection matrix, queue save/load serialization, ore draw pixel composition, and Rust code changes.
**Confidence:** High for the claimed TIBTRE placement slice.
**Active in YR:** Yes. `TerrainClass__AI @ 0x0071C730` calls `CellClass::SpreadTiberium @ 0x00483780`; `SpreadTiberium` calls `CellClass::PlaceTiberium @ 0x00487190`. Stock `rulesmd.ini` has `TIBTRE01/02/03 SpawnsTiberium=yes`, `IsAnimated=yes`, and `AnimationProbability=.003`.

## Working Notes

Target question: For TIBTRE-spawned ore, what does `CellClass::PlaceTiberium` write to density, overlay type/data, dirty state, and growth/spread queues?

Non-goals: Do not re-investigate TIBTRE animation timing, target rejection gates beyond the existing-overlay distinction, or native save/load of queues.

Evidence needed to mark COMPLETE: Decompile plus caller/callee evidence for `SpreadTiberium -> CanPlaceTiberium -> PlaceTiberium`, `PlaceTiberium` branch writes, queue helper calls, radar dirty call, and current Rust delta in `src/sim/terrain_spawn.rs`.

Stop conditions: Stop once the TIBTRE call path's branch is proven, empty/existing target behavior is resolved, and Rust-facing acceptance scenarios are stated.

## 1. Overview

TIBTRE spawning does not add density to an already-ore neighbor. `SpreadTiberium(force=1)` still validates each candidate with `CanPlaceTiberium`, and `CanPlaceTiberium` requires `OverlayTypeIndex == -1`; only an empty target reaches `PlaceTiberium`. The reached branch creates a new overlay variant, adds the cell to the growth queue, writes `OverlayData = 3`, dirties tactical/radar terrain, and does not add the new cell to the spread queue.

## 2. Class Layout / Key Offsets

| Owner | Offset | Role | TIBTRE-path use | Active in YR |
|---|---:|---|---|---|
| `CellClass` | `+0x24` | packed map coord | Passed to overlay constructor, queue helper, and radar dirty. | Yes; decompile `0x00487190`. |
| `CellClass` | `+0x44` | overlay type index | `CanPlaceTiberium` requires `-1`, so existing ore/overlay cells are skipped before TIBTRE placement. | Yes; decompile `0x004838E0`. |
| `CellClass` | `+0x11C` | slope index | TIBTRE target path requires flat via `CanPlaceTiberium`; sloped `PlaceTiberium` branch is not reached. | Yes for gate; conditional for non-TIBTRE callers. |
| `CellClass` | `+0x11E` | overlay data / density byte | New TIBTRE placement writes exact byte `3`. | Yes; decompile `0x00487190`. |
| `TiberiumClass` | `+0xE0` | image/base overlay type pointer | Flat overlay variant selection starts from `Image->ArrayIndex`. | Yes; decompile `0x00487190`, `rulesmd.ini [Riparius] Image=1`. |
| `TiberiumClass` | `+0xE4` | max density | Entry rejects `density >= MaxDensity`; grow-existing branch clamps to `MaxDensity - 1`. | Yes; decompile `0x00487190`. |
| `TiberiumClass` | `+0x10C/+0x110/+0x114/+0x118` | growth queue state | New TIBTRE cell is inserted immediately through `AddToGrowthQueue`. | Yes; decompile `0x007235A0`. |
| `TiberiumClass` | `+0xF0/+0xF4/+0xF8/+0xFC` | spread queue state | Not updated by the TIBTRE new-cell branch. Only the grow-existing branch calls `AddToSpreadQueue`. | Conditional; decompile `0x00487190`, `0x00722AF0`. |
| `RadarClass` | `+0x1228/+0x1234/+0x14D9` | dirty coord list/count/flag | New TIBTRE placement calls `RadarClass::MarkTerrainDirty`. | Yes; decompile `0x006551C0`. |

## 3. Core Logic

### 3.1 TIBTRE call path reaches only empty-cell germination

`CellClass::SpreadTiberium @ 0x00483780` handles `force=1` by defaulting missing source tiberium type to type `0` and then scanning the eight neighbors from a random start index. For each neighbor it calls `CellClass::CanPlaceTiberium @ 0x004838E0`; it calls `CellClass::PlaceTiberium(tib_type, 3)` only after that target returns true.

`CanPlaceTiberium @ 0x004838E0` requires `CellClass+0x44 == -1`. This means an existing ore overlay on a neighbor rejects the candidate before `PlaceTiberium` is called.

Active in YR: Yes. Evidence: decompile `0x00483780` plus callee list for `0x00483780`; caller list shows `TerrainClass__AI @ 0x0071C730` calls `SpreadTiberium`.

### 3.2 Density and max/clamp behavior

For the TIBTRE-reached branch, `PlaceTiberium @ 0x00487190` receives density argument `3`. The entry guard rejects only if `param_3 >= TiberiumClass+0xE4`; stock `MaxDensity` is `12`, so density `3` passes. The new-cell branch then writes `CellClass+0x11E = 3` exactly. There is no `-1` conversion and no post-write clamp in this branch.

The grow-existing branch in `PlaceTiberium` does add `param_3` to current `OverlayData` and clamps to `MaxDensity - 1` (`11` stock), but TIBTRE spread does not reach that branch because existing-overlaid targets fail `CanPlaceTiberium`.

Active in YR: Yes for the new-cell write through TIBTRE; conditional for grow-existing behavior through other callers such as `CellClass::GrowTiberium`. Evidence: decompile `0x00487190`; caller list for `0x00487190`.

### 3.3 Overlay type and data mapping

For flat new placement, `PlaceTiberium` allocates an `OverlayClass`, rolls `Random::RandomRanged(0, 0xB)`, and selects:

`g_OverlayTypeClass_Array[TiberiumClass.Image.ArrayIndex + random_0_to_11]`

Then it calls `OverlayClass::Constructor(selected_overlay_type, &coord, -1)`, adds the cell to the growth queue, and writes `OverlayData = 3`.

Stock YR `rulesmd.ini` has `[Riparius] Image=1`; the flat ore overlay range in `[OverlayTypes]` includes `TIB01` through `TIB12` starting at index `105`. TIBTRE with no source ore defaults to tiberium type `0` (`Riparius`), so standard TIBTRE creates one random flat Riparius overlay variant and data byte `3`.

Active in YR: Yes. Evidence: decompile `0x00483780`, decompile `0x00487190`, `rulesmd.ini` `[Riparius]`, `[OverlayTypes]`, and `TIBTRE01/02/03`.

### 3.4 Dirty and recalculation side effects

The TIBTRE new-cell branch calls tactical dirty rectangle logic and `RadarClass::MarkTerrainDirty @ 0x006551C0`. The radar helper deduplicates the coordinate in its dirty list, appends when needed, and sets `RadarClass+0x14D9 = 1`.

`PlaceTiberium` does not directly call `CellClass::RecalcAttributes`. Any land/passability update is owned by the overlay constructor or later recalc paths, not by an explicit `RecalcAttributes` call in `0x00487190`.

Active in YR: Yes. Evidence: callee list for `0x00487190`, decompile `0x00487190`, decompile `0x006551C0`; absence of `RecalcAttributes` from `0x00487190` callees.

### 3.5 Queue effects

The TIBTRE new-cell branch calls `TiberiumClass::AddToGrowthQueue @ 0x007235A0` immediately after overlay construction and before writing `OverlayData=3`. `AddToGrowthQueue` appends an entry `{coord, currentFrame + (signed_abs(Random::Next()) % 50)}`, stores that priority as a float, inserts it into the growth heap, increments the growth count, and sets the growth bitmap for that cell, provided the current overlay data is `< 11`.

The TIBTRE new-cell branch does not call `AddToSpreadQueue`. `AddToSpreadQueue @ 0x00722AF0` is called by the grow-existing branch of `PlaceTiberium`, not by the branch TIBTRE reaches.

Active in YR: Yes for new-cell growth enqueue through TIBTRE; conditional for spread enqueue through non-TIBTRE grow-existing calls. Evidence: decompile `0x00487190`, decompile `0x007235A0`, decompile `0x00722AF0`, callee list for `0x00487190`.

## 4. INI Keys

| Key | Location | Stock YR value | Binary effect | Active in YR |
|---|---|---:|---|---|
| `SpawnsTiberium` | `[TIBTRE01/02/03]` | `yes` | Makes terrain tree use the terrain-spawn path. | Yes. |
| `IsAnimated` | `[TIBTRE01/02/03]` | `yes` | Required by `TerrainClass::AI` animation/spawn path. | Yes. |
| `AnimationRate` | `[TIBTRE01/02/03]` | `3` | Timing before this report's placement slice. | Yes, out of scope here. |
| `AnimationProbability` | `[TIBTRE01/02/03]` | `.003` | Probability before this report's placement slice. | Yes, out of scope here. |
| `Image` | `[Riparius]` | `1` | Base flat overlay image range for type `0`. | Yes. |
| `Growth` / `GrowthPercentage` | `[Riparius]` | `2200` / `.06` | Later growth queue behavior after insertion. | Yes, later processing out of scope. |
| `Spread` / `SpreadPercentage` | `[Riparius]` | `2200` / `.06` | Later spread queue behavior; not inserted by TIBTRE new placement. | Yes, later processing out of scope. |

## 5. Integration Points

| Point | Evidence | Active in YR |
|---|---|---|
| TIBTRE reaches `SpreadTiberium` | Caller list for `0x00483780`: `TerrainClass__AI @ 0x0071C730`. | Yes. |
| `SpreadTiberium` reaches `PlaceTiberium` | Decompile/callee list for `0x00483780`; call passes density `3`. | Yes. |
| Empty target gate | `CanPlaceTiberium @ 0x004838E0` requires `OverlayTypeIndex == -1`. | Yes. |
| New overlay construction | `PlaceTiberium @ 0x00487190` calls `OverlayClass__Constructor @ 0x005FC380`. | Yes. |
| Growth queue insertion | `PlaceTiberium @ 0x00487190` calls `TiberiumClass__AddToGrowthQueue @ 0x007235A0`. | Yes. |
| Radar dirty | `PlaceTiberium @ 0x00487190` calls `RadarClass__MarkTerrainDirty @ 0x006551C0`. | Yes. |
| Current Rust TIBTRE placement | `src/sim/terrain_spawn.rs` `place_tiberium_empty`. | Partially implemented: existing overlays/resources are skipped and `OverlayData=3` is written; random flat variant selection and native growth queue insertion are still missing. |

## 6. Current Rust Implementation Status

As of the 2026-05-24 TIBTRE implementation pass, `src/sim/terrain_spawn.rs` no longer models TIBTRE placement as additive growth on existing ore. It skips existing resources/overlays before placement, creates new cells with `SPAWN_DENSITY_LEVELS=3`, and writes `OverlayData=3`.

Rust deltas for this slice:

- Existing ore neighbor behavior is now aligned for the current overlay/resource-grid path: binary TIBTRE skips existing-overlaid cells before `PlaceTiberium`, and Rust skips existing resources/overlays.
- New-cell overlay data is now aligned: binary writes `OverlayData=3`, and Rust writes overlay data `3`.
- New-cell overlay type selection remains incomplete. Binary randomizes among the tiberium type's 12 flat variants; Rust uses one `default_ore_overlay_id` resolved as the first overlay name starting with `TIB`.
- Queue effects are missing. Binary adds the new cell to the per-type growth queue immediately and does not add it to spread queue in the new-cell branch. Rust has no YR per-type queue model.
- Dirty metadata is partially present. `OverlayGrid::place_overlay` and `set_overlay_data` push dirty cells, and app code drains them later, but the binary distinction between new-cell radar dirty and grow-existing no-radar-dirty is not modeled.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TerrainClass::AI -> SpreadTiberium` path activation | verified | caller list `0x00483780`; prior `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md`; `rulesmd.ini` TIBTRE sections | Slot 1 owns exact animation timing. |
| `SpreadTiberium(force=1)` density argument | verified | decompile `0x00483780`, call to `0x00487190` with `3` | none |
| Existing-overlay target skip | verified | decompile `0x00483780`, `0x004838E0` requiring `+0x44 == -1` | Full target rejection matrix belongs to slot 3. |
| `PlaceTiberium` new-cell branch | verified | decompile `0x00487190` | none for TIBTRE path |
| `PlaceTiberium` grow-existing branch | verified for contrast | decompile `0x00487190`; caller list includes `CellClass::GrowTiberium` | Not reached from TIBTRE spread. |
| Overlay random flat variant | verified | decompile `0x00487190`; `rulesmd.ini [Riparius] Image=1` | Exact array-index construction for all four tiberium types already covered by prior spread report. |
| Growth queue insertion | verified | decompile `0x00487190`, `0x007235A0` | Save/load details out of scope. |
| Spread queue non-insertion on new cell | verified | decompile/callee ordering `0x00487190`; `0x00722AF0` only in grow branch | Later growth can feed spread queue, out of scope. |
| Radar dirty and Recalc absence | verified | callee list/decompile `0x00487190`, decompile `0x006551C0` | Exact renderer/minimap frame not covered. |
| Current Rust `place_tiberium_empty` | verified-source-scan | `src/sim/terrain_spawn.rs` | Random flat variant selection and native growth queue insertion remain. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this exhaustive or coverage-map? -> exhaustive-slice for TIBTRE-reached PlaceTiberium effects only.` (evidence: user slot scope and report header)
- `[RESOLVED] OQ-02 - Does TIBTRE reach PlaceTiberium in standard YR? -> yes, through TerrainClass::AI -> SpreadTiberium -> PlaceTiberium.` (evidence: caller list `0x00483780`; decompile `0x00483780`; `rulesmd.ini [TIBTRE01]`)
- `[RESOLVED] OQ-03 - What density argument is passed? -> exact integer `3`.` (evidence: decompile `0x00483780`)
- `[RESOLVED] OQ-04 - Can TIBTRE grow an existing ore neighbor? -> no; SpreadTiberium requires target CanPlaceTiberium, and that requires no existing overlay.` (evidence: decompile `0x00483780`, `0x004838E0`)
- `[RESOLVED] OQ-05 - What happens on an empty flat target? -> construct random flat overlay variant, add to growth queue, write OverlayData=3, dirty tactical/radar, return success.` (evidence: decompile `0x00487190`)
- `[RESOLVED] OQ-06 - What is the max/clamp rule? -> new TIBTRE branch has entry reject only if density >= MaxDensity; grow-existing branch clamps sum to MaxDensity-1.` (evidence: decompile `0x00487190`)
- `[RESOLVED] OQ-07 - Is a data frame of `levels - 1` correct for TIBTRE placement? -> no; binary writes data byte equal to the density argument, `3`. Current Rust now writes `3`.` (evidence: `0x00487190`; Rust `src/sim/terrain_spawn.rs`)
- `[RESOLVED] OQ-08 - Does new TIBTRE placement add to growth queue? -> yes, immediately, with `currentFrame + (signed_abs(Random::Next()) % 50)` priority stored as a float.` (evidence: decompile `0x00487190`, `0x007235A0`)
- `[RESOLVED] OQ-09 - Does new TIBTRE placement add to spread queue? -> no, not in the reached branch.` (evidence: decompile `0x00487190`, `0x00722AF0`)
- `[RESOLVED] OQ-10 - Does PlaceTiberium call RecalcAttributes? -> no direct call is present.` (evidence: callee list for `0x00487190`)
- `[RESOLVED] OQ-11 - Does new TIBTRE placement dirty radar? -> yes; `RadarClass::MarkTerrainDirty` is called and sets dirty flag `+0x14D9`.` (evidence: decompile `0x00487190`, `0x006551C0`)
- `[RESOLVED] OQ-12 - What Rust surface owns the remaining mismatch? -> `src/sim/terrain_spawn.rs` placement metadata plus future queue/overlay variant surfaces; existing-ore skip and `OverlayData=3` are already implemented.` (evidence: source scan)
- `[DEFERRED] OQ-13 - Exact direction-offset cardinal order for the eight-neighbor scan.` (category: out-of-scope; reason: slot 4 only needs placement effects after candidate selection; next-step-if-pursued: inspect `g_DirectionOffsets` initializer)
- `[DEFERRED] OQ-14 - Exact TIBTRE animation midpoint/RNG timing.` (category: out-of-scope; reason: slot 1 owns timing; next-step-if-pursued: use slot 1 report)
- `[DEFERRED] OQ-15 - Native save/load handling of growth queue entries.` (category: requires-different-system-context; reason: queue serialization is not placement side effects; next-step-if-pursued: dedicated TiberiumClass save/load queue pass)

Adversarial checks:

- If all eight neighbors already have ore overlays, TIBTRE places nothing.
- If the chosen valid neighbor is empty and flat, it receives `OverlayData=3`, not frame `2`.
- If a non-TIBTRE direct caller invokes `PlaceTiberium(type, 3)` on existing matching ore, the grow branch adds and clamps, but this is not the TIBTRE spread path.
- If the new cell is placed, it is eligible for future growth via growth queue insertion but is not immediately a spread queue source from this call.
- If radar dirty dedup already contains the coord, `MarkTerrainDirty` returns early before appending; otherwise it appends the coord and sets the dirty flag.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| TIBTRE `SpreadTiberium(force=1)` only calls `PlaceTiberium` on a target that passes `CanPlaceTiberium`, including no existing overlay. | `0x00483780`, `0x004838E0` | implemented for current resource/overlay-grid placement path | `src/sim/terrain_spawn.rs::can_accept_tiberium`, `try_spawn_ore` | Preserve existing-resource/overlay rejection for TIBTRE placement; if no empty valid neighbor exists, no spawn occurs. | `tibtre_spawn_skips_existing_ore_neighbors_instead_of_growing_them` | Do not model TIBTRE as additive ore growth. |
| New TIBTRE placement writes exact `OverlayData=3`. | `0x00483780`, `0x00487190` | implemented for current overlay-grid placement path | `src/sim/terrain_spawn.rs::place_tiberium_empty`; future shared `PlaceTiberium` primitive | Preserve overlay data byte `3` while keeping the economic density model aligned with verified `Reduce_Tiberium` semantics. | `tibtre_new_cell_overlay_data_is_three_not_stock_level_minus_one` | Do not derive runtime `PlaceTiberium` data as `remaining / base - 1`. |
| New TIBTRE placement chooses a random flat overlay variant from the tiberium type image range. | `0x00487190`; `rulesmd.ini [Riparius] Image=1`; `[OverlayTypes] TIB01..TIB12` | mismatch: Rust uses one `default_ore_overlay_id`. | terrain spawner config; overlay/tiberium type metadata | Store enough tiberium type/overlay metadata to choose one of the 12 flat Riparius variants with the binary-equivalent RNG range. | `tibtre_new_cell_randomizes_riparius_flat_overlay_variant` | Do not always place the first `TIB*` overlay id. |
| New TIBTRE placement immediately adds the cell to growth queue and does not add it to spread queue. | `0x00487190`, `0x007235A0`, `0x00722AF0` | missing: no YR per-type growth/spread queues for this path. | future `src/sim/ore_growth.rs` queue model; `ProductionState` | Insert new TIBTRE cells into the proper per-type growth queue with `currentFrame + (signed_abs(Random::Next()) % 50)` priority; leave spread queue unchanged at placement time. | `tibtre_new_cell_enqueues_growth_queue_not_spread_queue` | Do not use immediate spread candidates or the old RA1 scanner as a substitute for queue membership. |
| New TIBTRE placement dirties tactical/radar terrain, while `PlaceTiberium` does not directly call `RecalcAttributes`. | `0x00487190`, `0x006551C0`; callee list for `0x00487190` | partial: `OverlayGrid` dirty path exists, radar distinction not modeled. | `src/sim/overlay_grid.rs`, app dirty drain/minimap update boundary | Ensure new TIBTRE overlay mutation publishes dirty terrain/passability/minimap work at the same tick boundary without adding a sim dependency on render. | `tibtre_new_cell_marks_overlay_dirty_for_passability_and_minimap` | Do not update only `resource_nodes`; invisible or stale-passability ore is wrong. |
| Grow-existing `PlaceTiberium(type, 3)` adds/clamps and calls spread queue, but it is not reached by TIBTRE spread. | `0x00487190`; caller list for `0x00487190` | Rust conflates this branch into TIBTRE behavior. | shared future `PlaceTiberium` primitive vs terrain-spawn caller | Keep direct grow-existing semantics separate from TIBTRE target selection. | `place_tiberium_existing_branch_clamps_but_tibtre_never_uses_it_on_neighbors` | Do not make one helper's permissive additive behavior available to the TIBTRE caller unless the caller already passed binary target gates. |

### Negative Facts / Do Not Do

- Do not let TIBTRE increase density on an already-ore neighbor.
- Do not set new TIBTRE overlay data to `2` for a "three level" spawn; binary writes byte `3`.
- Do not pick a single default `TIB*` overlay id for every new TIBTRE cell.
- Do not enqueue a newly placed TIBTRE cell into the spread queue during the placement call.
- Do not add a direct `RecalcAttributes` call to mirror `PlaceTiberium`; the binary function does not call it.

### Stale Docs / Follow-up Docs

- `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md` should replace "Increases density by 3 if ore already exists" for the TIBTRE path with: "TIBTRE `SpreadTiberium(force=1)` skips existing-overlaid neighbors through `CanPlaceTiberium`; only empty valid targets reach `PlaceTiberium(type, 3)`. The grow-existing branch exists in `PlaceTiberium` for other callers."

## 10. Remaining Uncertainty

- Exact direction-offset cardinal label order remains out of scope. This report only requires the fact that all eight neighbors are scanned from a random start.
- Queue save/load behavior remains out of scope. The placement-time insertion into growth queue is verified, but native serialization/rebuild behavior is a separate target.
- Exact economic harvest amount for a newly placed `OverlayData=3` cell should be reconciled with the existing `Reduce_Tiberium` reports during implementation. This slot proves the data byte and queue side effects, not a new payment formula.

## Sources

- Ghidra decompile: `0x00483780` `CellClass::SpreadTiberium`
- Ghidra decompile: `0x004838E0` `CellClass::CanPlaceTiberium`
- Ghidra decompile: `0x00487190` `CellClass::PlaceTiberium`
- Ghidra decompile: `0x007235A0` `TiberiumClass::AddToGrowthQueue`
- Ghidra decompile: `0x00722AF0` `TiberiumClass::AddToSpreadQueue`
- Ghidra decompile: `0x006551C0` `RadarClass::MarkTerrainDirty`
- Ghidra caller/callee evidence: callers for `0x00483780`, callers/callees for `0x00487190`
- Prior reports: `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md`, `CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`, `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini` `[TIBTRE01/02/03]`, `[Riparius]`, `[OverlayTypes]`
- Rust scanned: `src/sim/terrain_spawn.rs`, `src/sim/overlay_grid.rs`, `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_queue.rs`
