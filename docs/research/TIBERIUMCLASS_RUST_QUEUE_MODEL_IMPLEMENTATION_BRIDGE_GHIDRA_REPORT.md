# TiberiumClass Rust Queue Model Implementation Bridge - Ghidra Research Report

**Address(es):** `0x0055AFB0`, `0x00686B20`, `0x007221B0`, `0x00722440`, `0x00722AF0`, `0x00722AB0`, `0x00722240`, `0x007228B0`, `0x00722C40`, `0x00722F00`, `0x007235A0`, `0x007233A0`, `0x00722D00`, `0x00480A80`, `0x00487190`  
**Investigation Mode:** coverage-map implementation bridge  
**Target question:** What current Rust surfaces must change to move from the scan/reservoir ore model toward GameMD per-`TiberiumClass` growth/spread queues without regressing the just-fixed TIBTRE lifecycle path?  
**Non-goals:** Do not re-decompile the queue processors, do not implement Rust, do not resolve native save/load stream layout, and do not re-open settled TIBTRE placement/damage facts unless a contradiction appears.  
**Evidence needed to mark COMPLETE:** Existing verified binary docs for queue ownership/tick/add contracts, current Rust file:line evidence for ownership/hash/save/tick surfaces, stale-doc corrections, concrete tests and do-not-do risks.  
**Stop conditions:** Stop after an implementation bridge is precise enough for a future patch; defer exact native save/load and duplicate-growth-entry proof to their dedicated re-swarm slots.

## 1. Scope Result

This slot is COMPLETE as an implementation bridge, not as a full binary investigation. Existing verified queue docs are strong enough to define the Rust migration shape. The remaining unknowns are intentionally left to other re-swarm slots: exact `GrowthProcessor` minutiae, duplicate `AddToGrowthQueue` reachability, native save/load, and map-load queue seeding.

## 2. Load-Bearing Verified Facts

1. **GameMD owns ore scheduling per `TiberiumClass`, not per map scan.** Active in YR: Yes. Evidence: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:37-60` records per-class spread/growth entry counts, heap pointers, bitmaps, entry arrays, and timers; `:84-101` records init/rebuild allocation and filtering by `TiberiumClass+0x98`.

2. **Live tick order is growth driver first, spread driver second.** Active in YR: Yes for standard skirmish when scenario growth/spread gate is enabled. Evidence: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:66-119` cites `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, growth driver `0x00722C40`, spread driver `0x007221B0`, and the `ScenarioClass+0x34A6` gate.

3. **Growth and spread processors are heap/batch processors, not cursor scans.** Active in YR: Yes for tiberium classes with positive percentages; stock Cruentus exits because percentages are zero. Evidence: spread processor batch is `heapCount * SpreadPercentage`, clamped `[5,25]`, then `Random % batch + 1`, with no priority wake-up gate and source reinsert only when valid-neighbor count is `>1` (`TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:121-142`). Growth processor batch is clamped `[5,50]`, grows before density check, reinserts still-growable cells with `currentFrame + Random % 50`, then feeds spread queue (`:144-163`).

4. **Runtime add helpers are asymmetric.** Active in YR: Yes. `AddToSpreadQueue @ 0x00722AF0` checks `CanSpreadTiberium` and spread bitmap before appending; `AddToGrowthQueue @ 0x007235A0` requires `OverlayData < 11` and appends `{coord, currentFrame + Random % 50}` while setting growth bitmap (`TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:165-188`). The existing report leaves exotic duplicate growth entries deferred (`:283-285`).

5. **TIBTRE new-cell placement must add growth queue before writing density byte 3 and must not add spread queue.** Active in YR: Yes for stock TIBTRE reaching the new-cell branch. Evidence: `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md:76-82` and implementation handoff at `:168`; dirty/radar side effects are at `:68-74`.

## 3. Current Rust Bridge Facts

1. **Rust still executes the old scan/reservoir processor.** Active in YR: No, this is Rust-only drift. Evidence: [ore_growth.rs](src/sim/ore_growth.rs:1) explicitly says RA1 algorithm; [ore_growth.rs](src/sim/ore_growth.rs:309) scans chunks from `scan_cursor`, reservoir-samples candidates, and executes only at full scan wrap.

2. **Rust now has partial native-shaped queue state, but it is not the live processor.** Active in YR: Rust-only partial bridge. Evidence: `OreGrowthQueueEntry` and `OreSpreadQueueEntry` exist at [ore_growth.rs](src/sim/ore_growth.rs:117); `OreGrowthState` stores `growth_queue`, `spread_queue`, and `spread_membership` at [ore_growth.rs](src/sim/ore_growth.rs:163). The tick path at [ore_growth.rs](src/sim/ore_growth.rs:309) does not pop or process those queues.

3. **TIBTRE currently enqueues growth after visible writes, so exact `PlaceTiberium` side-effect order still drifts.** Active in YR: No, Rust-only drift against active GameMD. Evidence: Rust inserts `resource_nodes`, places overlay/data, then calls `enqueue_growth_queue_cell` at [terrain_spawn.rs](src/sim/terrain_spawn.rs:541), while GameMD calls `AddToGrowthQueue` before writing `OverlayData=3` (`TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md:76-82`).

4. **Rust now hashes partial ore-growth scheduler state.** Active in YR: Rust deterministic lockstep concern; native `ComputeCRC` is not a queue serialization proxy. Evidence: [world_hash.rs](src/sim/world/world_hash.rs:179) calls `ore_growth_state.hash_state`; [ore_growth.rs](src/sim/ore_growth.rs:277) hashes scanner state plus partial growth/spread queues. Older queue report lines `223-239` are stale on this point.

5. **Rust snapshots currently serialize `ProductionState`, including the wrong scheduler shape.** Active in YR: Rust-only save model until native save/load is resolved. Evidence: `ProductionState` derives `Serialize, Deserialize` at [production_types.rs](src/sim/production/production_types.rs:196), owns `ore_growth_state` at [production_types.rs](src/sim/production/production_types.rs:207), and the queue report still defers native save/load at `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:309-313`.

6. **Full reduction now has a partial spread-reseed bridge.** Active in YR: Rust-only partial bridge for active GameMD reduction semantics. Evidence: [tiberium/mod.rs](src/sim/tiberium/mod.rs:87) clears overlay/resource, then calls `reseed_spread_neighbors_after_reduction`; the binary contract for full removal clearing spread membership then reseeding neighbors is in `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:190-196`.

7. **Current world tick placement is close but not equivalent to GameMD queue ownership.** Active in YR: Rust-only orchestration. Evidence: [world/mod.rs](src/sim/world/mod.rs:1644) runs old ore growth before TIBTRE spawners, and [world/mod.rs](src/sim/world/mod.rs:1653) says a spawn cannot be grown/spread until next tick. GameMD queue drivers run growth then spread from logic tick; TIBTRE AI is another live object path, so exact object-order relation needs the parent trace set, but the ore scheduler itself must not be a scan wrap.

## 4. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Per-`TiberiumClass` growth/spread queues with heaps, bitmaps, and timers; processors batch by percentage and consume RNG as documented. Active in YR: Yes (`0x00722240`, `0x007228B0`, `0x00722D00`, `0x007233A0`, `0x00722440`, `0x00722F00`; queue report lines `37-60`, `121-163`). | Replace `scan_cursor`/reservoir live execution with queue-backed processors; keep any transitional queue entry types only if they become the source of truth. | `src/sim/ore_growth.rs`, `src/sim/production/production_types.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`. | One Riparius cell below max density in growth queue is popped, grown, reinserted with jitter if still `<11`, and feeds spread queue; stock Cruentus queue processor exits at `GrowthPercentage=0`. | `growth_processor_pops_grows_reinserts_and_feeds_spread_queue`; `cruentus_processor_exits_when_percentages_zero`. | Extending the current scan model will keep RNG/order drift even if visible ore sometimes matches. |
| `PlaceTiberium` new-cell branch calls `AddToGrowthQueue` before writing `OverlayData=3`; no spread queue insertion. Active in YR: Yes (`TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md:76-82`). | Shared placement primitive must make queue insertion order explicit; TIBTRE helper currently enqueues after resource/overlay writes. | `src/sim/terrain_spawn.rs`, future shared `CellClass::PlaceTiberium`-style helper, `src/sim/overlay_grid.rs`. | TIBTRE midpoint creates new ore, consumes overlay variant RNG then growth queue RNG, inserts growth queue priority before density write is observed by subsequent side effects, and does not enqueue spread. | `tibtre_new_cell_queue_insert_precedes_overlay_data_write`; `tibtre_new_cell_enqueues_growth_not_spread`. | If order is left implicit, later dirty/radar/queue consumers will drift under traces that inspect intermediate state. |
| Full ore removal clears spread bitmap membership for the removed cell in every tib type, then reseeds eligible same-type neighbors into the removed type's spread queue. Active in YR: Yes (`0x00480A80`, `0x00722AB0`, `0x00722AF0`; queue report lines `190-196`). | Current reduction reseeds same-type neighbors but only has a `BTreeSet` and simplified queue entries; it needs per-type bitmap and priority/jitter matching `AddToSpreadQueue`. | `src/sim/tiberium/mod.rs`, `src/sim/ore_growth.rs`, miner/combat/smudge reduction callers. | Harvest fully removes a Riparius cell, clears all per-type spread membership for that coord, enqueues eligible neighboring Riparius cells with current-frame-plus-jitter priorities, and changes world hash. | `reduce_tiberium_full_removal_clears_all_type_spread_bits_and_reseeds_same_type_neighbors`. | Direct placement or candidate insertion cannot reproduce later heap order or dedup behavior. |
| Queue state affects future deterministic output and must be hashed/serialized or rebuilt only under a proven native save/load contract. Active in YR: Yes for deterministic future behavior; native save/load exact stream still unresolved. Evidence: queue report lines `287-298`, `309-313`; current Rust hash at [world_hash.rs](src/sim/world/world_hash.rs:179). | Keep hash coverage, but change it from scanner/partial queues to canonical per-type timers, heaps, entries, bitmaps/membership, and config fields. Snapshot should serialize implemented queue state until a later report proves rebuild-on-load. | `src/sim/world/world_hash.rs`, `src/sim/snapshot.rs`, `src/sim/production/production_types.rs`. | Two sims with same visible `resource_nodes` but different growth heap order produce different world hashes and round-trip through snapshot without queue loss. | `ore_growth_queue_order_changes_world_hash`; `ore_growth_queue_state_round_trips_through_snapshot`. | Hashing only visible ore makes desyncs latent until a later growth/spread tick. |

## 5. Negative Facts / Do Not Do

- **Do not keep the RA1 scan/reservoir algorithm as the live YR model.** Active in YR: No. Evidence: Rust RA1 docs at [ore_growth.rs](src/sim/ore_growth.rs:1); GameMD queue ownership at `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:37-60`.
- **Do not interpret queue priority as a "sleep until frame" gate.** Active in YR: No such gate found. Evidence: spread/growth processor details at `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:135-162`.
- **Do not implement depletion reseed as immediate ore placement or as a future scan candidate.** Active in YR: No. Evidence: `Reduce_Tiberium` reseed contract at `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:190-196`.
- **Do not hardcode only Riparius/ore as queue-capable.** Active in YR: Conditional by per-type percentages. Evidence: stock `[Tiberiums]` and Cruentus zero percentages at `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:198-210`; Rust currently short-circuits non-ore in [ore_growth.rs](src/sim/ore_growth.rs:343).
- **Do not drop queue state on Rust snapshot restore unless native save/load research proves GameMD rebuilds it.** Active in YR: Unresolved. Evidence: native save/load uncertainty at `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:309-313`; Rust serde state at [production_types.rs](src/sim/production/production_types.rs:196).

## 6. Proposed Rust Tests

- `growth_processor_pops_grows_reinserts_and_feeds_spread_queue`
- `spread_processor_reinserts_source_only_when_multiple_valid_neighbors_remain`
- `tibtre_new_cell_queue_insert_precedes_overlay_data_write`
- `tibtre_new_cell_enqueues_growth_not_spread`
- `reduce_tiberium_full_removal_clears_all_type_spread_bits_and_reseeds_same_type_neighbors`
- `ore_growth_queue_order_changes_world_hash`
- `ore_growth_queue_state_round_trips_through_snapshot`
- `cruentus_processor_exits_when_percentages_zero`

## 7. Stale-Doc Replacement Wording Found

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:223-239` should be replaced with: "Current Rust still executes a scan/reservoir processor, but it now also stores partial native-shaped growth/spread queue entries in `OreGrowthState`, serializes them through `ProductionState`, and hashes them via `WorldHash`. These partial queues are not yet the live GameMD-equivalent processor."
- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:280-281` should be replaced with: "`OreGrowthState` is now hashed and serialized, but the hashed/serialized state remains transitional because the live scheduler is still scan/reservoir plus partial queues, not the full per-`TiberiumClass` heap/bitmap/timer model."
- `docs/research/PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md:188-196` should be replaced with: "Rust now parses theater `AllowTiberium` and stores `allows_tiberium` in resolved terrain, but the old `ore_growth::can_germinate` scan path still does not use the full `CanPlaceTiberium` gate chain."
- `docs/research/TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md:168` should be replaced with: "Rust now has a partial `enqueue_growth_queue_cell` bridge for TIBTRE placement, but the call order is still after resource/overlay writes rather than the GameMD order before `OverlayData=3`, and the queue is not yet processed by a GameMD-equivalent growth processor."

## 8. Remaining Uncertainty

- Exact native save/load stream behavior for queue entries, bitmaps, timers, and post-load rebuild remains unresolved by this slot.
- Duplicate `AddToGrowthQueue` reachability remains unresolved; existing decompile says no growth bitmap guard in the helper, but caller-specific duplicate prevention belongs to another slot.
- Exact map-load queue seeding interaction with current Rust `resource_nodes`/`overlay_grid` construction belongs to the map-load seeding slot.
- Exact object/tick ordering between TIBTRE terrain AI and global queue drivers should remain governed by the trace reports already produced; this bridge only covers replacing the ore scheduler model.

## 9. Sources

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`
- `docs/research/TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`
- `docs/research/PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`
- Current Rust scans: `src/sim/ore_growth.rs`, `src/sim/tiberium/mod.rs`, `src/sim/terrain_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/sim/production/production_types.rs`, `src/sim/overlay_grid.rs`, `src/app_init.rs`, `src/map/theater.rs`, `src/map/resolved_terrain.rs`.
