# YR Tiberium Boundary Design

## Goal

Implement standard Yuri's Revenge tiberium reduction, harvest, growth, spread, and placement as one deterministic sim-owned subsystem instead of split miner/combat/growth helpers.

## Architecture Context

Current Rust stores ore/gem economic stock in `ProductionState::resource_nodes`, visual overlay state in `OverlayGrid`, harvest extraction in `src/sim/miner/miner_system.rs`, combat/smudge ore damage in `miner::reduce_tiberium`, and growth/spread in `src/sim/ore_growth.rs`. These pieces currently agree well enough for simple harvesting, but they do not share the same mutation boundary.

The verified YR behavior does have a shared gameplay boundary: `CellClass::Reduce_Tiberium` mutates the cell overlay state, returns the exact removed density amount, dirties terrain/radar/tactical surfaces, and interacts with `TiberiumClass` spread/growth queue state. Standard harvester code, combat ore damage, and spread/growth code all need the same underlying tiberium state to stay deterministic.

Existing architecture constraints:

- `sim/` must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- `ProductionState` already owns deterministic economy state and is serialized.
- `OverlayGrid` already stores overlay id/data and dirty cells.
- `Simulation::state_hash` hashes resource nodes and overlay grid, but not current `OreGrowthState`.
- Existing sim systems use explicit function boundaries and deterministic `BTreeMap` iteration.

## TS/YR Boundary Guard

This design intentionally uses YR's own legacy data names, not TS gameplay as a shortcut. `rulesmd.ini` still defines `[Tiberiums]`, `Riparius`, `Cruentus`, `TiberiumGrows`, and `TiberiumSpreads`, and verified `gamemd.exe` paths still route standard ore/gem reduction, harvesting, growth, and spread through `TiberiumClass`-named code.

In scope for standard YR:

- Data-driven `[Tiberiums]` parsing and stock `Riparius`/`Cruentus` ore/gem metadata.
- Queue-backed growth/spread for types whose YR rules percentages allow it.
- `PlaceTiberium`/`CanPlaceTiberium` behavior used by YR ore spread and TIBTRE terrain spawning.
- Harvest/reduction behavior used by stock `HARV` and `CMIN`.

Out of scope for this implementation unless separately verified and requested:

- TS `Weeder`/weed harvesting rules and weed overlay behavior.
- Treating Vinifera/Aboreus as stock-map visible behavior without seeded YR overlay/map evidence.
- TS fog/vein-style systems or other inherited code paths that are present in the binary but not active in standard YR.

## Impact Analysis

Touched modules:

- `src/sim/tiberium/` new module for shared tiberium state, rules metadata, reduction, placement, and queue processing.
- `src/sim/production/production_types.rs` to replace or wrap `resource_nodes`, `ore_growth_config`, and `ore_growth_state`.
- `src/sim/production/production_queue.rs` for map overlay seeding into the new tiberium state.
- `src/sim/miner/miner_system.rs` to call the shared reduction API instead of `extract_bales_max`.
- `src/sim/miner/mod.rs` to keep miner-specific cargo/config and stop owning global `reduce_tiberium`.
- `src/sim/combat/mod.rs` and `src/sim/combat/smudge_dispatch.rs` to call the shared reduction API.
- `src/sim/ore_growth.rs` to replace the RA1 scan/reservoir implementation with YR queue processing or move logic under `sim/tiberium`.
- `src/sim/world/mod.rs` for tick ordering and dirty-result application.
- `src/sim/world/world_hash.rs` for queue state hashing.
- `src/map/theater.rs` / resolved terrain data for `AllowTiberium`.
- `src/rules/ruleset.rs` or a new rules submodule for `[Tiberiums]` data.

Risk areas:

- Determinism: queue entries, bitmaps, timers, and RNG consumption must be hashed and serialized.
- Save compatibility: replacing `OreGrowthState` needs serde defaults or migration-safe layout.
- Tick order: YR growth runs before spread; existing TIBTRE terrain spawners currently run after ore growth.
- Passability/radar dirty timing: gamemd recalculates attributes inside full reduction; Rust must publish equivalent sim-owned dirty effects before later dependent decisions.
- Data model migration: current `ResourceNode { resource_type, remaining }` cannot alone express `OverlayData` semantics for the `11` return edge.

## Chosen Approach

Approach A: full shared tiberium boundary replacement.

2026-05-24 brainstorm confirmation: this remains the selected approach after the
Chrono Miner refinery dock/unload synthesis. The alternative staged/miner-only
paths still leave verified normal-play tiberium parity holes, so implementation
should follow the full boundary plan rather than a local harvest-only patch.

Create `src/sim/tiberium/` as the owner for tiberium-type metadata, cell resource state, `Reduce_Tiberium`-equivalent mutation, queue-backed growth/spread state, `PlaceTiberium`-equivalent placement, and deterministic hash/serde support.

Miner, combat, smudge, terrain-spawn, and growth/spread code become callers of this subsystem. `OverlayGrid` remains the overlay storage owner, but tiberium operations mutate it through narrow APIs and emit sim-owned dirty consequences. App/render layers continue consuming overlay/radar dirty state without `sim` depending on render.

This is preferred because the verified bug is not only "CMIN gets 12 instead of 11." The player-visible system includes future regrowth/spread timing, visible overlay frames, radar/minimap dirties, ore depletion, combat crater depletion, and deterministic replay state.

## Tiny-Detail Ledger

- `Reduce_Tiberium(20)` on `OverlayData=11` returns `11`, not `12`. Source: `REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`, `0x00480A80`.
- Partial reduction subtracts requested amount from `OverlayData` and returns the requested amount when `amount < current + 1`. Source: `0x00480A80`.
- `amount <= 0` returns `0` with no mutation; signed guard matters even if Rust callers normally pass unsigned values. Source: `0x00480A80`.
- Full reduction order is overlay type clear, overlay data zero, `RecalcAttributes`, radar dirty, all-type spread bitmap clear, same-type neighbor spread reseed, tactical dirty. Source: `0x00480A80`, `0x0047D2B0`, `0x006551C0`, `0x00722AB0`, `0x00722AF0`.
- The density-11 `AddToGrowthQueue` detour is a net no-op for `OverlayData=11`, because the callee sees unchanged value `11` and admits only `< 11`. Source: `0x00480A80`, `0x007235A0`.
- Standard YR owns growth and spread as per-`TiberiumClass` queue state: entry arrays, heaps, membership bitmaps, and per-type timers. Source: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`.
- Map load seeds growth and spread queues after overlay load and all-cell recalc. Source: `0x00686B20`, `0x00722D00`, `0x00722240`.
- Live tick calls growth before spread. Source: `0x0055AFB0`.
- Spread processor batch count is `clamp(ftol(heap_count * SpreadPercentage), 5, 25)`, then `Random % batch + 1`; processor reinsert priority is `0.0`. Source: `0x00722440`.
- Growth processor batch count is `clamp(ftol(heap_count * GrowthPercentage), 5, 50)`, then `Random % batch + 1`; still-growable cells reinsert with `currentFrame + Random % 50` and feed spread queue. Source: `0x00722F00`.
- Runtime spread calls `PlaceTiberium(tib_type, 3)`, so new spread cells get `OverlayData=3`, not `0`. Source: `0x00483780`, `0x00487190`.
- New spread overlay type is randomized from `Image->ArrayIndex + Random(0..11)`, not copied from the source overlay id. Source: `0x00487190`.
- Spread target validation is `CanPlaceTiberium`: playfield, bridge mask, live visible building, `SpawnsTiberium` terrain object, buildable land type, no existing overlay, flat slope, and theater `AllowTiberium`. Source: `0x004838E0`.
- Standard spread target path cannot reach sloped `PlaceTiberium` branch because `CanPlaceTiberium` requires flat target. Source: `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`.
- Stock Cruentus/gems do not spread because `[Cruentus] SpreadPercentage=0`, not because all gems are hardcoded dormant. Source: `ini/rulesmd.ini [Cruentus]`, `0x00722440`.
- `TiberiumGrowthEnabled` scenario byte gates spread and growth drivers; Rust must not apply it only to growth if YR gates both. Source: `0x007221B0`, map `[Basic]`.
- Harvester state 1 calls `Harvest_Ore_Tick` after 9 StepTimer steps; stock `HarvesterLoadRate=2` gives first extraction at 18 frames. Source: `0x0073E5E0`, `RulesClass+0x1520`.
- Destination-present harvest ticks return success without extraction and without timer reset. Source: `0x0073D450`.
- Full cargo at the harvest gate does not call `Reduce_Tiberium`; return logic wins before continuation short scan. Source: `0x0073D450`.
- Request amount is `ftol(Storage - total_storage)`; preserve truncation toward zero for normal and future fractional storage cases. Source: `0x0073D450`, `0x007C5F00`.
- Gems use the same harvest branch as ore but store Cruentus amount/value, with stock `Value=50`. Source: `0x00485010`, `ini/rulesmd.ini [Cruentus]`.
- Standard `HARV` and `CMIN` do not use the TS `Weeder` branch. Source: `0x0073D450`, `ini/rulesmd.ini [HARV]/[CMIN]`.

## Design

### Components

#### `sim::tiberium::types`

Owns deterministic gameplay data types:

- `TiberiumTypeId(u8)` or similar compact id.
- `TiberiumKind` / legacy mapping for stock `Riparius`, `Cruentus`, and future rules-defined types.
- `TiberiumTypeDef` parsed from `[Tiberiums]` and each named section:
  - `name`
  - `image_index`
  - `value`
  - `growth`
  - `growth_percentage`
  - `spread`
  - `spread_percentage`
  - `max_density` defaulted from verified binary behavior, with parser support if later proven data-driven.
- `TiberiumCellState` keyed by cell:
  - tiberium type id
  - economic/storage density value used by `Reduce_Tiberium`
  - optional bridge to the visible overlay id/data.

Initial implementation can keep `ResourceNode` as a compatibility view, but authoritative reduction should consult overlay data or `TiberiumCellState`, not `remaining / base` alone.

#### `sim::tiberium::state`

Owns serialized, hashable runtime state:

- `cells: BTreeMap<(u16, u16), TiberiumCellState>` or a compatibility wrapper around `ProductionState::resource_nodes`.
- `queues_by_type: BTreeMap<TiberiumTypeId, TiberiumQueueState>`.
- Per-type growth/spread timers.
- Per-type spread and growth queue membership bitmaps.
- Ordered queue entries with deterministic priority ordering.

Queue representation should be idiomatic Rust, not a byte-for-byte heap clone, but it must reproduce:

- membership/dedup effects,
- pop/reinsert order,
- RNG consumption,
- timer reload behavior,
- hash/serde stability.

#### `sim::tiberium::reduce`

Exports one authoritative reduction function:

```rust
pub(crate) fn reduce_tiberium(
    ctx: &mut TiberiumMutationCtx<'_>,
    cell: (u16, u16),
    amount: i32,
) -> TiberiumReduction;
```

`TiberiumMutationCtx` should hold only sim-owned state:

- mutable tiberium cells / resource nodes,
- mutable `OverlayGrid`,
- mutable queue state,
- terrain/passability dirty accumulator,
- radar terrain dirty accumulator,
- map dimensions / placement helpers.

`TiberiumReduction` should include:

- removed density amount,
- tiberium type id,
- removed economic value source,
- whether full removal occurred,
- dirty cells/results needed by world/app boundary.

The API accepts signed `amount` internally to preserve the `amount <= 0` no-op guard even if callers pass unsigned values.

#### `sim::tiberium::place`

Exports YR `PlaceTiberium`-equivalent helpers:

- `can_place_tiberium(cell, env) -> bool`
- `place_tiberium(ctx, cell, type_id, density) -> PlaceTiberiumResult`
- `spread_tiberium_from(ctx, source_cell, type_id)`

For standard spread, `place_tiberium(type, 3)` must:

- randomize flat overlay variant from tiberium type image range,
- set overlay data to `3`,
- insert the new cell into growth queue,
- dirty overlay/radar/tactical equivalent state,
- avoid sloped target branch because target validation rejects slopes.

#### `sim::tiberium::queue`

Replaces the current RA1 scan/reservoir `ore_growth.rs` behavior with:

- `seed_queues_from_overlays`
- `tick_growth_queues`
- `tick_spread_queues`
- `add_to_growth_queue`
- `add_to_spread_queue`
- `clear_spread_bitmaps_all_types_for_cell`
- `rebuild_growth_queue`
- `rebuild_spread_queue`

Queue processors must run growth before spread from `Simulation::advance_tick`.

#### Rules and Map Data

Rules parser adds a `[Tiberiums]` parser and stores tiberium definitions in `RuleSet`.

Theater parser adds `AllowTiberium` to tileset metadata and exposes it through resolved terrain cells or a lookup helper usable from `sim`.

Overlay seeding maps ore/gem overlay ids to tiberium type ids using verified overlay/tiberium image mapping. Do not globally change visual overlay frame semantics just to fix the `11` return; the reduction helper must preserve visible overlay id/data while computing the gamemd return result.

### Interfaces / Contracts

Miner contract:

- Miner harvest asks the tiberium subsystem to reduce current cell by truncated empty capacity.
- Miner receives removed density amount and tiberium type/value.
- Miner cargo updates remain miner-owned; cell mutation is no longer miner-owned.
- Full-cargo and destination-present gates happen before reduction.
- Retarget/return remains state-machine-owned, not reduction-owned.

Combat/smudge contract:

- Combat/smudge call the same reduction helper with their damage-derived amount.
- They do not directly mutate `resource_nodes`.
- Overlay, queue, radar, and dirty consequences are identical to miner depletion where gamemd uses the same helper.

Growth/spread contract:

- Queue state owns future behavior and must be serialized and hashed.
- Growth processor may mutate existing cells and feed spread queue.
- Spread processor may call `place_tiberium(type, 3)` on validated targets.
- Stock gems remain non-spreading because data percentages are zero.

World/app boundary contract:

- `sim` emits or stores dirty terrain/radar facts in sim-owned structures.
- App/render drains those facts as today, but no render/ui dependency enters `sim`.
- If passability recalc must be observable before a later same-tick sim query, world must apply the sim-owned dirty consequence before that query.

### Data Flow

Map load:

1. Parse `[Tiberiums]`, tiberium sections, overlay registry, theater `AllowTiberium`, map overlay packs.
2. Seed tiberium cells from overlay id/data.
3. Recalc overlay/passability as already done by map initialization.
4. Seed per-type growth/spread queues from overlay-backed tiberium cells.
5. Hashable state is now complete before first sim tick.

Harvest:

1. Miner state machine checks destination-present, full cargo, and harvest timer gates.
2. It computes request amount from empty storage capacity.
3. It calls `tiberium::reduce_tiberium`.
4. It adds returned amount into cargo/storage type.
5. It resets timer, returns, or short-scans according to `Harvest_Ore_Tick`/`Mission_Harvest` ordering.

Reduction full removal:

1. Resolve tiberium type from cell overlay/tiberium state.
2. Preserve signed no-op guards.
3. If partial, subtract overlay data and return amount.
4. If full, return pre-removal `OverlayData`, clear overlay, publish terrain/radar dirty, clear all-type spread bitmap for that cell, reseed eligible neighbors into removed type's spread queue.

Growth/spread tick:

1. For each tiberium type, process growth driver if interval fires.
2. Process growth heap batch and update cells/queues.
3. For each tiberium type, process spread driver if interval fires.
4. Process spread heap batch and call validated `place_tiberium(type, 3)` where appropriate.
5. TIBTRE terrain spawners remain after ore growth unless later binary evidence requires integration into queue timing.

### Error Handling

- Missing tiberium metadata for an overlay should fail closed for mutation and log a debug warning; do not invent `Riparius` silently unless verified fallback behavior says so.
- Missing overlay grid in tests should allow resource-only tests only where the test explicitly constructs a reduced context; production gameplay should have overlay data for tiberium cells.
- Missing `AllowTiberium` data should default according to parsed theater fallback only after verified parser behavior; until then, tests should exercise both allowed and rejected cells.
- Unsupported native save/load queue behavior remains an explicit uncertainty. Rust snapshots should serialize queue state until a later save/load investigation proves rebuild-on-load parity.

### Testing Strategy

Unit tests:

- `reduce_tiberium_overlaydata_11_full_removal_returns_11`
- `reduce_tiberium_partial_and_zero_amount_match_gamemd`
- `reduce_tiberium_full_removal_recalc_and_reseeds_neighbors`
- `tiberium_queue_state_changes_world_hash`
- `ore_growth_queue_state_round_trips_through_snapshot`
- `ore_growth_seeds_per_type_growth_and_spread_queues_from_overlay_data`
- `spread_processor_uses_clamped_percentage_batch_and_zero_priority_reinsert`
- `growth_processor_reinserts_still_growable_cell_and_feeds_spread_queue`
- `yr_spread_germination_places_density_three_ore_cell`
- `yr_spread_germination_randomizes_flat_riparius_overlay_variant`
- `yr_spread_rejects_clear_walkable_tile_when_allow_tiberium_false`
- `yr_spread_rejects_tibtree_occupied_cell`
- `yr_stock_cruentus_spread_percentage_zero_skips_spread_processor`

Miner integration tests:

- `cmin_first_harvest_gate_is_18_frames`
- `harvest_tick_with_destination_present_does_not_extract`
- `full_miner_on_ore_returns_without_reducing_cell`
- `cmin_overlaydata_11_extracts_11_bales`
- `cmin_gem_overlaydata_11_extracts_11_gem_bales_value_550`
- `empty_current_cell_short_scans_only_after_harvest_gate`
- `standard_harv_and_cmin_are_not_weeder_path`

Hash/serde tests:

- Two sims with identical visible ore but different queue membership must hash differently.
- Queue timers, entries, bitmaps, and tiberium cell state round-trip through serde.
- Overlay-only dirty cells must not be used as queue membership.

Regression tests:

- Combat crater ore reduction and miner harvest must call the same reduction path.
- Smudge dispatch ore damage must not leave stale overlay/passability/radar state.

## Architectural Decisions

- New module rather than extending `miner`: tiberium is map/cell gameplay, not miner-specific behavior.
- Keep `sim` independent of render: dirty terrain/radar effects are represented as sim-owned state/events.
- Prefer data-driven tiberium types over hardcoded ore/gem branching: stock Cruentus is data-disabled for growth/spread, but mods can use other percentages.
- Preserve deterministic collections: use `BTreeMap` for keyed state and canonical queue ordering for hash/serde.
- Do not globally reinterpret overlay frame `11` as economic level `12`; full-removal return semantics depend on the current overlay data byte.
- Serialize queue state now; defer native save/load rebuild semantics until a dedicated investigation.

## Alternatives Considered

### B. Staged Boundary First, Queue Later

This would create a shared reduction helper first but keep the current RA1 scan/reservoir growth model temporarily. It reduces implementation size, but leaves verified normal-play YR growth/spread drift in place: no per-type queues, no bitmaps, wrong spread density, wrong overlay variant, and missing queue hash/serde state.

Rejected as the chosen design because the user explicitly asked to go deeper and the re-swarm proved the queue state is part of the same player-visible tiberium relationship.

### C. Chrono Miner Patch Only

This would fix harvest timing and the `11` cargo return only in `extract_bales_max`.

Rejected. It would leave combat/smudge ore damage on a stale helper, keep growth/spread parity wrong, and preserve the split mutation boundary that caused the mismatch.

## Remaining Uncertainty

- Exact native save/load behavior for tiberium queue arrays/bitmaps/timers is still open.
- Exact cardinal labels for `g_DirectionOffsets[0..7]` are deferred; current reports prove table use and wrapped random-start order.
- Exact API boundary for synchronous `RecalcAttributes` parity needs implementation-time design against current path/passability update flow.
- Selected-unit cargo pip rendering for `11/20` CMIN storage remains out of scope.
- x87 sub-1.0 fractional storage request behavior remains a low-frequency edge unless fractional storage is modeled.

## Stale Docs Suggested

- `docs/contracts/2026-05-23-chrono-miner-reduce-tiberium-implementation-contract.md`: replace broad "density mapping, 11-vs-12 bale behavior" wording with the verified statement that full-removal harvest amount is pre-removal `OverlayData`; for `OverlayData=11`, `Reduce_Tiberium(20)` returns `11`, not `12`.
- `src/sim/ore_growth.rs` module comment should stop claiming RA1 scanner parity once this design is implemented.
- `miner::reduce_tiberium` comments should stop claiming it mirrors gamemd `CellClass::Reduce_Tiberium` until it is replaced by the shared tiberium boundary.

## Handoff

Use `docs/plans/2026-05-23-yr-tiberium-boundary-plan.md` as the execution plan
for this design. The implementation should start at Task 1 and keep the stop
conditions intact; do not jump straight to the miner harvest fix before the
tiberium type metadata, `AllowTiberium`, state skeleton, hash, and serde
boundaries are in place.
