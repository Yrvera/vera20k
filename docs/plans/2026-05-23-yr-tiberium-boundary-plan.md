# YR Tiberium Boundary Implementation Plan

> For Codex: Execute this plan task-by-task. Each task is self-contained. Do not skip the tests for a task before moving to the next one.

**Goal:** Replace split ore/gem behavior with a shared standard-YR tiberium subsystem covering `Reduce_Tiberium`, map overlay seeding, harvest extraction, combat/smudge reduction, queue-backed growth/spread, `PlaceTiberium(type, 3)`, and deterministic hash/serde state.

**Architecture:** Add a sim-owned `tiberium` module. Keep `ProductionState::resource_nodes` as a compatibility surface during migration, but make the new tiberium state and helpers authoritative for all new reduction, placement, and queue behavior. `sim` must not depend on render/ui/sidebar/audio/net.

**Design Doc:** [docs/plans/2026-05-23-yr-tiberium-boundary-design.md](2026-05-23-yr-tiberium-boundary-design.md)

---

## Grounding Summary

- `Reduce_Tiberium(20)` on `OverlayData=11` returns `11`, not `12`; full removal clears overlay type/data, recalcs attributes, dirties radar/tactical, clears all-type spread bitmap membership, then reseeds same-type neighbors. Source: `docs/research/REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md`.
- Standard YR tiberium growth/spread is per-`TiberiumClass` queue state with heaps, entries, membership bitmaps, and per-type timers. Source: `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`.
- Runtime spread calls `PlaceTiberium(tib_type, 3)`, writes `OverlayData=3`, selects a random flat overlay variant from the type image range, and uses `CanPlaceTiberium`, not generic walkability. Source: `docs/research/PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`.
- Standard `HARV`/`CMIN` harvest uses `Harvest_Ore_Tick`: first extraction gate after `9 * HarvesterLoadRate`, destination-present skip, full-cargo no-reduce, request amount `ftol(Storage-total)`, gems as Cruentus value `50`, no TS Weeder path. Source: `docs/research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md`.
- Current Rust splits ownership across `ProductionState::resource_nodes`, `OverlayGrid`, miner `extract_bales_max`, combat/smudge `miner::reduce_tiberium`, and RA1-style `OreGrowthState`; current `world_hash.rs` omits `ore_growth_state`. Source: `docs/research/CURRENT_RUST_TIBERIUM_INTEGRATION_GAPS_AND_OWNERSHIP_GHIDRA_REPORT.md`.

## TS/YR Boundary Guard

The implementation must not import TS-only gameplay just because the binary and INI names say "tiberium." Standard YR still uses legacy `TiberiumClass` and `[Tiberiums]` names for ore/gems, so those names are in scope only where the verified YR reports mark the paths active.

Implement:

- YR `[Tiberiums]` parsing and data-driven `Riparius`/`Cruentus` ore/gem metadata.
- Growth/spread queue state for YR types allowed by their YR percentages.
- `PlaceTiberium`/`CanPlaceTiberium` behavior used by YR ore spread and TIBTRE spawning.
- Stock `HARV`/`CMIN` harvest and reduction behavior.

Do not implement in this plan:

- TS `Weeder`/weed harvesting rules for stock `HARV`/`CMIN`.
- Stock-map assumptions for Vinifera/Aboreus unless YR map/overlay seeding proves visible use.
- TS fog, vein, or other inherited systems that are merely present in `gamemd.exe`.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Add | [src/rules/tiberium_type.rs](../../src/rules/tiberium_type.rs) | Parse `[Tiberiums]` and per-type sections into data-driven defs |
| Modify | [src/rules/ruleset.rs](../../src/rules/ruleset.rs) | Store tiberium definitions on `RuleSet` |
| Modify | [src/map/theater.rs](../../src/map/theater.rs) | Parse and expose `AllowTiberium` |
| Modify | [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) | Carry per-cell `allow_tiberium` or expose lookup |
| Add | [src/sim/tiberium/mod.rs](../../src/sim/tiberium/mod.rs) | Module entry and public internal API |
| Add | [src/sim/tiberium/types.rs](../../src/sim/tiberium/types.rs) | Type ids, cell state, queue entries, mutation results |
| Add | [src/sim/tiberium/state.rs](../../src/sim/tiberium/state.rs) | Serialized/hashable tiberium runtime state |
| Add | [src/sim/tiberium/reduce.rs](../../src/sim/tiberium/reduce.rs) | Shared `Reduce_Tiberium` equivalent |
| Add | [src/sim/tiberium/place.rs](../../src/sim/tiberium/place.rs) | `CanPlaceTiberium`, `SpreadTiberium`, `PlaceTiberium` equivalent |
| Add | [src/sim/tiberium/queue.rs](../../src/sim/tiberium/queue.rs) | Growth/spread queue seeding and tick processing |
| Modify | [src/sim/mod.rs](../../src/sim/mod.rs) | Register `tiberium` module |
| Modify | [src/sim/production/production_types.rs](../../src/sim/production/production_types.rs) | Store new tiberium state/config alongside compatibility nodes |
| Modify | [src/sim/production/production_queue.rs](../../src/sim/production/production_queue.rs) | Seed tiberium cells from overlays |
| Modify | [src/app_init.rs](../../src/app_init.rs) | Initialize tiberium config/state after rules/map/overlay load |
| Modify | [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) | Replace `extract_bales_max` caller with shared reduction and fix gates |
| Modify | [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) | Demote or remove old `reduce_tiberium` helper |
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Use shared reduction helper for ore damage |
| Modify | [src/sim/combat/smudge_dispatch.rs](../../src/sim/combat/smudge_dispatch.rs) | Use shared reduction helper for crater/smudge ore reduction |
| Modify | [src/sim/ore_growth.rs](../../src/sim/ore_growth.rs) | Replace RA1 scanner or forward to `sim::tiberium::queue` |
| Modify | [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | Tick growth before spread and apply dirty/radar consequences |
| Modify | [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) | Hash tiberium queue/cell state |
| Modify | relevant tests | Add acceptance tests named below |

## Key Technical Decisions

- **New `sim::tiberium` module, not miner-owned behavior.** Tiberium is cell/map gameplay and has non-miner callers.
- **Compatibility first.** Keep `resource_nodes` during migration so existing scan/search/deposit code can compile while new tiberium state becomes authoritative.
- **Overlay data is authoritative for `Reduce_Tiberium` return edges.** Do not fix `OverlayData=11` by changing only `remaining` math.
- **Queue state is gameplay state.** Serialize and hash queue entries, timers, and membership state.
- **Dirty effects stay sim-owned.** Do not call render/minimap/UI from sim; emit overlay/radar/passability dirty state through existing sim/app boundaries.
- **Data-driven tiberium types.** Stock gems do not spread because `[Cruentus] SpreadPercentage=0`, not because `ResourceType::Gem` is hardcoded out of queues.

## Open Questions

### Resolved During Planning

- **Can we implement only CMIN harvest?** No. It leaves combat/smudge and growth/spread parity holes.
- **Should map `OverlayData=11` seed 12 cargo units?** No for full-removal return. The shared reduction helper must return pre-removal `OverlayData` for full removal.
- **Can depletion reseed be direct ore placement?** No. It must clear/reseed queue membership.
- **Can spread use source overlay id and frame 0?** No. Runtime spread uses random flat variant and `OverlayData=3`.

### Deferred

- Exact native save/load queue stream behavior. Until researched, serialize exact Rust queue state.
- Exact direction table cardinal labels. Use existing 8-neighbor order only where reports prove order matters; add a follow-up if visual/path parity needs named labels.
- Exact synchronous passability recalc boundary. Initial implementation should emit deterministic dirty state and apply it before dependent sim queries; validate during implementation.
- Selected cargo pip rendering for 11/20 CMIN storage.
- x87 sub-1.0 fractional cargo edge unless fractional storage is introduced.

## Sim Checklist

- [x] No `sim` dependency on render/ui/sidebar/audio/net.
- [x] Deterministic keyed state uses sorted/canonical order.
- [x] Queue state planned for serde and `state_hash`.
- [x] Growth-before-spread tick order preserved.
- [x] `TiberiumGrowthEnabled` applies to spread as well as growth where binary gates both.
- [x] Old miner/combat/smudge direct helpers are migration targets, not final owners.
- [ ] If any new cached overlay/tiberium mapping state is added, include it in serde/hash.
- [ ] If passability recalc moves into sim, verify no render/app dependency is introduced.

## Risk Areas

- **Large signature migration:** Combat and smudge currently receive only `resource_nodes`; shared reduction needs overlay, queue state, dirty/radar accumulators. Mitigation: add a context object and update callers one at a time.
- **Double source of truth:** Keeping `resource_nodes` while adding tiberium state can drift. Mitigation: all mutations go through `sim::tiberium`; compatibility nodes are updated inside that boundary only.
- **Hash churn:** New state changes replay hashes. Mitigation: add explicit hash tests before wiring live processors.
- **RNG consumption drift:** Queue processors consume random numbers in verified order. Mitigation: write focused RNG/queue tests before full integration.
- **Spread validation incompleteness:** `AllowTiberium`, live building, spawner terrain, bridge flags, land buildable, overlay absence, flat slope all matter. Mitigation: implement validation helper with one test per gate.
- **Existing dirty files:** The worktree is dirty. Before each implementation task, read the files being modified and preserve unrelated changes.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | `[Tiberiums]` data and `AllowTiberium` are parsed | Spread type/value/variant/validation are data-driven | Parser unit tests |
| 2 | Tiberium state is serialized and hashed | Future spread/growth diverges if queue state is hidden | Hash/serde tests |
| 3 | Overlay seeding fills tiberium cells and queues | Map load is the first live tiberium state | Overlay-backed seed tests |
| 4 | Full/partial `Reduce_Tiberium` semantics | Fixes 11-vs-12 and shared full-removal side effects | Reduction unit tests |
| 4 | Full removal queue/dirty side effects | Depletion changes future spread and terrain state | Full removal reseed test |
| 5 | Combat/smudge use shared reduction | Crater/warhead ore damage must not diverge | Combat/smudge tests |
| 6 | Miner harvest gates call shared reduction | Player sees cargo, timing, return behavior | Miner integration tests |
| 7 | `PlaceTiberium(type, 3)` spread | New ore density/visual variant are visible | Spread placement tests |
| 8 | Queue growth/spread processors | Normal skirmish ore regrowth/spread timing | Queue processor tests |
| 8/10 | World tick order and dirty bridge | Same-tick determinism and app/render sync | World integration tests |

---

## Tasks

### Task 1: Parse Tiberium Type Metadata And `AllowTiberium`

**Why:** The subsystem must be data-driven before behavior is moved. Stock Riparius and Cruentus values, image ranges, and spread/growth percentages are not hardcoded constants.

**Files:**
- Add: [src/rules/tiberium_type.rs](../../src/rules/tiberium_type.rs)
- Modify: [src/rules/ruleset.rs](../../src/rules/ruleset.rs)
- Modify: [src/map/basic.rs](../../src/map/basic.rs) or the current special-flags owner if the parser has moved.
- Modify: [src/map/theater.rs](../../src/map/theater.rs)
- Modify: [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs)
- Modify tests in `src/rules/*tests.rs`, `src/map/theater_tests.rs`, or nearby existing test modules.

**Steps:**

1. Add `TiberiumTypeDef` with fields:
   - id/name,
   - image index,
   - value,
   - growth,
   - growth percentage,
   - spread,
   - spread percentage,
   - max density default `12`.
2. Parse `[Tiberiums]` ordered entries from merged rules, then parse each named section.
3. Store `Vec<TiberiumTypeDef>` or `BTreeMap<TiberiumTypeId, TiberiumTypeDef>` on `RuleSet`.
4. Add stock tests for `Riparius` and `Cruentus` using `ini/rulesmd.ini` values:
   - Riparius `Image=1`, `Value=25`, `Growth=2200`, `GrowthPercentage=.06`, `Spread=2200`, `SpreadPercentage=.06`.
   - Cruentus `Image=2`, `Value=50`, `Growth=10000`, `GrowthPercentage=0`, `Spread=10000`, `SpreadPercentage=0`.
5. Parse `[TileSetNNNN] AllowTiberium=` in `map/theater.rs`.
6. Parse and expose the global/special spread gate used by the binary's
   `SpecialFlags.TiberiumSpreads` path. Standard YR skirmish defaults to enabled,
   but the queue and TIBTRE spread drivers must be able to suppress spread when it
   is disabled.
7. Expose `allow_tiberium` in resolved terrain or a deterministic lookup helper available to sim validation.

**Tests:**

```text
cargo test parse_stock_tiberium_types_from_rulesmd
cargo test cruentus_percentages_zero_but_type_exists
cargo test theater_tileset_allow_tiberium_parsed
cargo test standard_rules_enable_tiberium_spreads_special_flag
```

**Stop conditions:**

- Do not start behavior changes until rules/map metadata tests pass.
- Do not start Task 3 until overlay array-index mapping is explicit and tested:
  map overlay pack numeric ids through the original overlay array indices, not
  through any compact Rust registry id. Verified stock ranges are `GEM01..GEM12`,
  `TIB01..TIB20`, `TIB2_01..TIB2_20`, and `TIB3_01..TIB3_20`.
- Preserve the verified `OverlayToTiberiumIndex` fallback: if an overlay is marked
  `Tiberium=yes` but is outside every configured type image range, resolve it to
  Riparius/type 0 rather than treating it as non-tiberium.

### Task 2: Add `sim::tiberium` State Skeleton

**Why:** Establish the authoritative state shape and serde/hash targets before moving live mutation callers.

**Files:**
- Add: [src/sim/tiberium/mod.rs](../../src/sim/tiberium/mod.rs)
- Add: [src/sim/tiberium/types.rs](../../src/sim/tiberium/types.rs)
- Add: [src/sim/tiberium/state.rs](../../src/sim/tiberium/state.rs)
- Modify: [src/sim/mod.rs](../../src/sim/mod.rs)
- Modify: [src/sim/production/production_types.rs](../../src/sim/production/production_types.rs)
- Modify: [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs)

**Steps:**

1. Add module docs explaining that this replaces YR tiberium cell/queue behavior and must not depend on render/UI.
2. Define:
   - `TiberiumTypeId`,
   - `TiberiumCellState`,
   - `TiberiumQueueEntry`,
   - `TiberiumTypeQueueState`,
   - `TiberiumState`.
3. Include serialized fields for:
   - cell map,
   - per-type growth/spread entries,
   - per-type membership bitmaps or canonical membership sets,
   - per-type timers.
4. Add `TiberiumState::new(map_width, map_height, type_count)`.
5. Add `ProductionState::tiberium_state`.
6. Keep existing `resource_nodes`, `ore_growth_config`, and `ore_growth_state` for compatibility at this task.
7. Add explicit hash coverage in `world_hash.rs`.

**Tests:**

```text
cargo test tiberium_state_default_is_empty
cargo test tiberium_queue_state_round_trips_through_snapshot
cargo test tiberium_queue_state_changes_world_hash
```

**Stop conditions:**

- Do not wire live callers until hash/serde coverage exists.
- Do not use `OverlayGrid::dirty_cells` as queue membership.

### Task 3: Seed Tiberium Cells And Queues From Map Overlays

**Why:** Live maps must initialize the new tiberium state before any harvester, combat, or growth tick consumes it.

**Files:**
- Add/modify: [src/sim/tiberium/queue.rs](../../src/sim/tiberium/queue.rs)
- Modify: [src/sim/production/production_queue.rs](../../src/sim/production/production_queue.rs)
- Modify: [src/app_init.rs](../../src/app_init.rs)
- Modify production tests.

**Steps:**

1. Add a seeding helper that reads map overlay entries, original overlay array ids,
   overlay names, and resolved tiberium type image ranges.
2. Resolve tiberium type by verified image-range membership first, using the
   original overlay array index from map data. Name prefixes are only a diagnostic
   fallback for tests and logging, not the authoritative mapping.
3. Cover stock Riparius, Cruentus, Vinifera, and Aboreus image ranges even though
   stock YR maps mostly exercise Riparius and Cruentus.
4. If an overlay is `Tiberium=yes` but does not fall inside any configured image
   range, seed it as Riparius/type 0 to match verified fallback behavior.
5. Seed `TiberiumCellState` using the actual overlay data byte.
6. Continue updating `resource_nodes` as a compatibility view, but mark the tiberium cell state as authoritative.
7. Seed growth/spread queues after overlay cells are loaded and after any existing all-cell terrain/passability initialization.
8. Seed spread queues only for cells that pass verified source eligibility; seed growth queues for growable cells.
9. Preserve existing `default_ore_overlay_id` behavior until `PlaceTiberium` random variant selection replaces it.

**Tests:**

```text
cargo test ore_growth_seeds_per_type_growth_and_spread_queues_from_overlay_data
cargo test overlay_frame_11_seeds_tiberium_cell_overlaydata_11
cargo test stock_gem_overlay_seeds_cruentus_type
cargo test overlaypack_original_tib2_tib3_indices_seed_correct_tiberium_types
cargo test tiberium_yes_overlay_outside_image_ranges_falls_back_to_riparius
```

**Stop conditions:**

- If the compatibility `resource_nodes` value disagrees with new cell state, all mutation after this point must update both through the tiberium module only.
- Do not declare `frame + 1` fixed here; the `Reduce_Tiberium` return logic lands in Task 4.

### Task 4: Implement Shared `Reduce_Tiberium` Equivalent

**Why:** This is the central player-visible bug: harvest, combat, and smudge need one authoritative mutation boundary.

**Files:**
- Add: [src/sim/tiberium/reduce.rs](../../src/sim/tiberium/reduce.rs)
- Modify: [src/sim/tiberium/mod.rs](../../src/sim/tiberium/mod.rs)
- Modify: [src/sim/overlay_grid.rs](../../src/sim/overlay_grid.rs) only if a narrow helper is needed.
- Add tests under `src/sim/tiberium/*` or a new test module.

**Steps:**

1. Define `TiberiumMutationCtx<'a>` containing mutable references to:
   - `TiberiumState`,
   - compatibility `resource_nodes`,
   - optional `OverlayGrid`,
   - radar/terrain dirty accumulator or callback,
   - map/tiberium metadata needed for queue reseed.
2. Define `TiberiumReduction` with:
   - `removed_amount`,
   - `type_id`,
   - `value_per_unit`,
   - `full_removal`,
   - dirty cells emitted.
3. Implement signed guard: `amount <= 0` returns no-op.
4. Resolve current `OverlayData` and tiberium type from new state/overlay.
5. Partial path:
   - condition `amount < current_overlay_data + 1`,
   - subtract amount from overlay data,
   - return amount,
   - update compatibility node.
6. Full path:
   - return pre-removal `OverlayData`,
   - clear overlay id/type and overlay data,
   - update compatibility node removal,
   - publish dirty terrain/radar facts,
   - clear spread membership for the removed cell in all types,
   - reseed valid neighbors into the removed type's spread queue.
7. Implement the density-11 growth detour as a no-op for `OverlayData=11` unless a non-11 case needs a direct call.

**Tests:**

```text
cargo test reduce_tiberium_overlaydata_11_full_removal_returns_11
cargo test reduce_tiberium_partial_and_zero_amount_match_gamemd
cargo test reduce_tiberium_full_removal_recalc_and_reseeds_neighbors
cargo test reduce_tiberium_full_removal_clears_all_type_spread_membership
```

**Stop conditions:**

- Do not wire miner/combat until these tests pass.
- If passability dirty cannot be applied before dependent sim queries, pause and design the exact `RecalcAttributes` boundary before live wiring.

### Task 5: Wire Combat And Smudge Through Shared Reduction

**Why:** Combat/crater ore damage currently uses `miner::reduce_tiberium`, which cannot update overlay/queue/radar side effects.

**Files:**
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs)
- Modify: [src/sim/combat/smudge_dispatch.rs](../../src/sim/combat/smudge_dispatch.rs)
- Modify: [src/sim/world/mod.rs](../../src/sim/world/mod.rs) if call signatures need wider context.
- Modify combat/smudge tests.

**Steps:**

1. Change combat/smudge entry points that reduce ore to receive a tiberium mutation context or enough world-level access to build one.
2. Replace direct calls to `crate::sim::miner::reduce_tiberium`.
3. Preserve existing crater ordering: crater path reduction happens before smudge placement attempt.
4. Assert overlay and queue side effects occur on combat/smudge full removal.
5. Audit all current Rust tiberium reduction call sites and the verified original
   caller families before declaring this task complete. Besides miner and crater
   paths, account for radius reduction, animation middle logic, chain-reaction
   paths, and any future system that removes tiberium from a cell.
6. Make the shared helper the only non-test path allowed to mutate tiberium
   density/overlay removal. Unimplemented original caller families should be
   recorded as explicit future integration points, not left as unknowns.
7. Leave old miner helper in place only for tests that have not migrated; mark it deprecated in comments or stop exporting it if all callers are migrated.

**Tests:**

```text
cargo test reduce_tiberium_authoritative_path_shared_by_harvest_and_crater
cargo test crater_ore_reduction_clears_overlay_and_reseeds_spread_queue
cargo test crater_path_reduces_ore_before_smudge_placement_failure
cargo test no_non_test_tiberium_reduction_call_sites_bypass_shared_helper
```

**Stop conditions:**

- If borrow constraints tempt a duplicate helper in combat/smudge, stop and restructure context passing instead.

### Task 6: Wire Miner Harvest Through Shared Reduction

**Why:** Miner harvest is the most visible player path and currently bypasses the helper.

**Files:**
- Modify: [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs)
- Modify: [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs)
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs)

**Steps:**

1. In `handle_harvest`, implement destination-present skip before timer/extraction if current Rust can observe an active movement destination in harvest state.
2. Fix first-gate timing so first extraction fires on the 18th frame at stock default, not one tick late.
3. Check full cargo before calling reduction; full cargo must leave ore unchanged and enter return flow before short scan.
4. Replace `extract_bales_max` call with `tiberium::reduce_tiberium` using request amount from empty capacity.
5. Convert removed amount and tiberium type/value into `CargoBale`s.
6. Preserve state-machine ownership:
   - reduction does not short-scan,
   - reduction does not choose next ore,
   - miner handles return/continuation after reduction result.
7. Keep `extract_bales_max` only as a migrated test helper if still needed; otherwise remove or make private to tests.

**Tests:**

```text
cargo test cmin_first_harvest_gate_is_18_frames
cargo test harvest_tick_with_destination_present_does_not_extract
cargo test full_miner_on_ore_returns_without_reducing_cell
cargo test cmin_overlaydata_11_extracts_11_bales
cargo test cmin_gem_overlaydata_11_extracts_11_gem_bales_value_550
cargo test empty_current_cell_short_scans_only_after_harvest_gate
cargo test standard_harv_and_cmin_are_not_weeder_path
```

**Stop conditions:**

- Do not change refinery unload/dock behavior here.
- Do not move retarget search into the tiberium helper.

### Task 7: Implement `CanPlaceTiberium` And `PlaceTiberium(type, 3)`

**Why:** Growth/spread cannot be wired until placement creates the right visible ore cell and queue side effects.

**Files:**
- Add/modify: [src/sim/tiberium/place.rs](../../src/sim/tiberium/place.rs)
- Modify: [src/sim/terrain_spawn.rs](../../src/sim/terrain_spawn.rs)
- Modify map/terrain tests if `AllowTiberium` fixtures are needed.

**Steps:**

1. Implement `can_place_tiberium` gates:
   - in playfield,
   - no bridge mask / bridge-blocking cell,
   - no live visible building on target,
   - no `SpawnsTiberium` terrain object on target,
   - land type is buildable,
   - no existing overlay,
   - flat slope,
   - theater `AllowTiberium`.
2. Implement `place_tiberium(ctx, cell, type_id, density)` for new-cell runtime spread.
3. For runtime spread, pass density `3`.
4. Select random flat overlay variant from tiberium type image base plus `Random(0..11)`.
5. Set `OverlayData=3`, update tiberium cell state and compatibility node.
6. Add the new cell to the same type's growth queue.
7. Dirty overlay/radar/terrain through sim-owned dirty state.
8. Implement the existing-cell growth path as a separate helper or explicit mode
   (`grow_tiberium_cell` / verified `PlaceTiberium` branch) rather than directly
   incrementing `OverlayData` from the queue processor.
9. Existing-cell growth must preserve the verified Branch A gate set and side
   effects:
   - `TiberiumGrowthEnabled` is true,
   - existing overlay resolves to a valid tiberium type,
   - target cell passes the flat/no-grow constraints checked by the binary,
   - density is below the verified growth limit,
   - the type's `GrowthPercentage` gate passes,
   - existing cell type matches the requested tiberium type,
   - compatibility node is updated,
   - Branch A does not call `RecalcAttributes` or `RadarClass::MarkTerrainDirty`;
     only emit the dirty/update facts that are verified for density growth,
   - spread-queue feed happens where verified.
10. Route TIBTRE / `SpawnsTiberium` terrain spawning through the same placement
    primitive and gates, while preserving the already-verified tick ordering.
11. Preserve the verified TIBTRE terrain AI gates before calling spread:
    `SpawnsTiberium=yes`, `IsAnimated=yes`, `TiberiumGrowthEnabled=true`, and
    spread-special flag enabled.

**Tests:**

```text
cargo test yr_spread_germination_places_density_three_ore_cell
cargo test yr_spread_germination_randomizes_flat_riparius_overlay_variant
cargo test yr_spread_rejects_clear_walkable_tile_when_allow_tiberium_false
cargo test yr_spread_rejects_tibtree_occupied_cell
cargo test yr_spread_rejects_existing_overlay_and_sloped_target
cargo test yr_stock_cruentus_spread_percentage_zero_skips_spread_processor
cargo test yr_growth_existing_cell_uses_place_tiberium_growth_side_effects
cargo test yr_growth_existing_cell_does_not_mark_radar_dirty
cargo test tibtree_spawn_uses_shared_tiberium_placement_gates
cargo test tibtree_spawn_requires_spawns_tiberium_and_is_animated
cargo test tibtree_spawn_disabled_when_tiberium_spreads_flag_false
```

**Stop conditions:**

- Do not copy source overlay id for runtime spread.
- Do not infer `AllowTiberium` purely from land type.

### Task 8: Replace RA1 Ore Growth Scanner With YR Queue Processors

**Why:** Normal skirmish regrowth/spread is queue-driven in YR; the current scan/reservoir implementation cannot reproduce queue membership, ordering, or RNG consumption.

**Files:**
- Add/modify: [src/sim/tiberium/queue.rs](../../src/sim/tiberium/queue.rs)
- Modify: [src/sim/ore_growth.rs](../../src/sim/ore_growth.rs)
- Modify: [src/sim/world/mod.rs](../../src/sim/world/mod.rs)
- Modify: [src/sim/production/production_types.rs](../../src/sim/production/production_types.rs)
- Modify ore growth tests.

**Steps:**

1. Implement queue operations:
   - `add_to_growth_queue`,
   - `add_to_spread_queue`,
   - `clear_spread_bitmaps_all_types_for_cell`,
   - `rebuild_growth_queue`,
   - `rebuild_spread_queue`.
2. Implement growth driver:
   - exact per-type timer state, including initial `last = -1` behavior,
     elapsed-frame comparison, last-frame update, and interval reload,
   - `clamp(ftol(heap_count * GrowthPercentage), 5, 50)`,
   - `Random % batch + 1`,
   - verified growth interval multiplier/reload behavior,
   - call the existing-cell growth helper from Task 7 instead of mutating density inline,
   - still-growable reinsert priority `currentFrame + Random % 50`,
   - feed spread queue where verified.
3. Implement spread driver:
   - exact per-type timer state, including initial `last = -1` behavior,
     elapsed-frame comparison, last-frame update, and interval reload from `Spread`,
   - exits when the spread-special flag equivalent to `SpecialFlags.TiberiumSpreads`
     is disabled,
   - `SpreadPercentage <= 0` exits,
   - `clamp(ftol(heap_count * SpreadPercentage), 5, 25)`,
   - `Random % batch + 1`,
   - no current-frame gate on heap priority,
   - call `SpreadTiberium` / `PlaceTiberium(type, 3)`,
   - reinsert source only under verified neighbor-count conditions.
4. Make `ore_growth::tick_ore_growth` either delegate to `tiberium::queue::tick` or remove the old scanner behind a compatibility function name.
5. Apply `TiberiumGrowthEnabled` so it gates both growth and spread where verified.
6. Apply the spread-special flag so it suppresses spread-driver and TIBTRE spread
   paths without suppressing density growth.
7. Preserve TIBTRE after-growth order while still routing terrain-spawn tiberium
   creation through the shared placement primitive from Task 7.

**Tests:**

```text
cargo test ore_growth_processor_runs_before_spread_processor_same_tick
cargo test spread_processor_uses_clamped_percentage_batch_and_zero_priority_reinsert
cargo test growth_processor_reinserts_still_growable_cell_and_feeds_spread_queue
cargo test yr_spread_source_with_one_valid_neighbor_not_reinserted_after_spread
cargo test yr_spread_source_with_two_valid_neighbors_reinserted
cargo test yr_tiberium_growth_enabled_false_suppresses_spread_driver
cargo test tiberium_spreads_flag_false_suppresses_spread_but_not_growth
cargo test growth_queue_initial_timer_state_matches_verified_first_fire
cargo test spread_queue_timer_reload_uses_verified_spread_interval
```

**Stop conditions:**

- Do not leave the old RA1 full-map scan active for standard YR queues.
- If RNG consumption cannot be matched from reports, pause and run a narrow `/re-investigate` before finalizing queue processor order.
- If implementation discovers duplicate growth enqueue paths or a conflict between
  canonical membership sets and observed `AddToGrowthQueue` behavior, pause and
  verify the caller/bitmap semantics before enforcing set-only behavior.

### Task 9: Finish Hash, Serde, And Compatibility Cleanup

**Why:** Once live callers use the new state, old compatibility surfaces should not remain authoritative or invisible to hash/serde.

**Files:**
- Modify: [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs)
- Modify: [src/sim/production/production_types.rs](../../src/sim/production/production_types.rs)
- Modify: [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs)
- Modify: [src/sim/ore_growth.rs](../../src/sim/ore_growth.rs)
- Modify tests.

**Steps:**

1. Confirm `TiberiumState` hash includes:
   - cell states,
   - queue entries in canonical order,
   - queue membership state,
   - timers,
   - relevant type ids/config references.
2. Confirm serde round-trip preserves queue ordering and membership.
3. Remove or clearly deprecate `miner::reduce_tiberium` after all call sites migrate.
4. Update stale comments:
   - `src/sim/ore_growth.rs` no longer claims RA1 parity,
   - old `miner::reduce_tiberium` no longer claims gamemd parity if retained as a compatibility wrapper.
5. Audit `rg "reduce_tiberium|extract_bales_max|resource_nodes|OverlayData|PlaceTiberium|SpawnsTiberium"` and ensure mutations outside `sim::tiberium` are intentional compatibility writes, read-only scans, or explicit future-integration stubs.

**Tests:**

```text
cargo test tiberium_queue_state_changes_world_hash
cargo test ore_growth_queue_state_round_trips_through_snapshot
cargo test no_legacy_miner_reduce_tiberium_call_sites
```

**Stop conditions:**

- Do not remove `resource_nodes` if unrelated systems still read it for ore search, AI, or terrain spawn. Remove only direct mutation ownership.

### Task 10: Focused End-To-End Verification

**Why:** This change touches a high-frequency gameplay loop. Unit correctness is not enough; run focused player-visible scenarios.

**Files:**
- Existing test modules under `src/sim/miner`, `src/sim/combat`, `src/sim/production`, `src/sim/tiberium`.

**Steps:**

1. Run focused parser/state tests.
2. Run focused tiberium reduction/queue tests.
3. Run miner harvest tests.
4. Run combat/smudge ore reduction tests.
5. Run world hash/serde tests.
6. Run a wider sim test subset if focused tests pass.

**Verification commands:**

```text
cargo test tiberium
cargo test reduce_tiberium
cargo test cmin_overlaydata_11_extracts_11_bales
cargo test harvest_tick_with_destination_present_does_not_extract
cargo test crater_ore_reduction
cargo test ore_growth
cargo test world_hash
```

**Manual smoke scenario:**

- Standard Allied CMIN harvests an `OverlayData=11` Riparius cell.
- First pickup occurs at 18 frames after harvest state entry.
- Cargo increases by 11 Riparius units worth 275 credits when deposited.
- Overlay clears on depletion.
- A neighboring eligible same-type spread source is queued.
- Combat/crater depletion of an ore cell uses the same side effects.

## Must Not Change

- CMIN outbound movement remains drive-to-ore, not warp.
- Close/far CMIN refinery return behavior remains governed by existing miner/refinery reports.
- Stock refinery unload/deposit/release behavior is not part of this plan.
- Standard `HARV`/`CMIN` do not use TS Weeder timing.
- Stock Cruentus gems remain non-spreading because data percentages are zero.
- `sim` does not call render/sidebar/ui/audio/net.

## Suggested Follow-Up Docs After Implementation

- Refresh `docs/contracts/2026-05-23-chrono-miner-reduce-tiberium-implementation-contract.md` with the final shared-boundary implementation evidence.
- Update stale ore-growth and smudge-system docs that describe the old RA1 scanner or `miner::reduce_tiberium` as authoritative.
- If save/load queue behavior matters, run a dedicated `/re-investigate "TiberiumClass save load queue serialization"` before claiming native save parity.
