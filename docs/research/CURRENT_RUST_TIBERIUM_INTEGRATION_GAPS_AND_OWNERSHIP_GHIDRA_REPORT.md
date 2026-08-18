# Current Rust Tiberium Integration Gaps And Ownership - Rust Coverage Report

**Date:** 2026-05-23  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Current Rust ownership, call sites, tick order, hash/serialization concerns, and handoff surfaces for implementing RA2/YR `Reduce_Tiberium` and tiberium queue parity.  
**Non-Scope:** New binary analysis, exact RA2/YR queue algorithm verification, Yuri Slave Miner, TS Weeder, selected-unit pip formula, and Rust code changes.  
**Confidence:** High for Rust evidence, Medium for binary behavior referenced from existing reports.  
**Active in YR:** Current Rust surfaces are active in loaded skirmish sims when the relevant systems are configured.

## 1. Overview

Current Rust has three separate tiberium ownership surfaces instead of one authoritative `Reduce_Tiberium` equivalent:

- `ProductionState::resource_nodes` owns economic stock by cell.
- `OverlayGrid` owns the live overlay byte used for ore rendering, wall/bridge overlay state, and overlay-driven passability recalculation.
- `OreGrowthState` owns an RA1-style incremental scan/reservoir growth model, not RA2/YR per-tiberium growth/spread queues.

The implementation risk is not just the 12-vs-11 cargo mismatch. A correct fix needs a single sim-facing ore reduction boundary that updates resource stock, overlay byte, passability/radar dirty state, and RA2/YR queue state in deterministic order.

## 2. Verified Rust Evidence

| Area | Current Rust fact | Evidence |
|---|---|---|
| Resource stock owner | `ProductionState` stores `resource_nodes: BTreeMap<(u16,u16), ResourceNode>` plus `ore_growth_config`, `ore_growth_state`, `terrain_spawners`, and `default_ore_overlay_id`. | `src/sim/production/production_types.rs:197-221` |
| Resource node model | `ResourceNode` is only `{ resource_type, remaining }`; it does not preserve original `OverlayData` separately. | `src/sim/miner/mod.rs:40-42` |
| Map overlay seeding | Live map ore seeding converts `entry.frame.min(11) + 1` into stock, so overlay frame 11 becomes 12 density levels. | `src/sim/production/production_queue.rs:132-170` |
| Miner harvest path | `handle_harvest` calls `extract_bales_max`; it does not call `reduce_tiberium`. | `src/sim/miner/miner_system.rs:520-534` |
| Miner extraction mutation | `extract_bales_max` reads `node.remaining / base`, removes `resource_nodes` on full drain, and calls `OverlayGrid::clear_overlay`; no queue/radar side effects are emitted there. | `src/sim/miner/miner_system.rs:804-855` |
| Combat/smudge reduction helper | `reduce_tiberium` lives in `miner/mod.rs`, claims to mirror `CellClass::Reduce_Tiberium`, but only mutates `resource_nodes`. | `src/sim/miner/mod.rs:390-424` |
| Current `reduce_tiberium` call sites | Smudge dispatch calls it for crater ore destruction; combat AoE also calls it for warhead tiberium damage. | `src/sim/combat/smudge_dispatch.rs:140`, `src/sim/combat/smudge_dispatch.rs:226`, `src/sim/combat/smudge_dispatch.rs:290`, `src/sim/combat/mod.rs:1116-1126` |
| Overlay mutation owner | `OverlayGrid::clear_overlay`, `place_overlay`, and `set_overlay_data` mutate overlay cells and push `dirty_cells`. | `src/sim/overlay_grid.rs:40-48`, `src/sim/overlay_grid.rs:92-118` |
| Dirty passability timing | App layer drains `OverlayGrid::take_dirty_cells()` after `advance_tick` and calls `recalc_overlay_passability`; this is not synchronous inside sim reduction. | `src/app_sim_tick.rs:679-692` |
| Radar dirty surface | `Simulation` has `mark_radar_terrain_dirty_cells`, currently used by bridge repair orders, not by ore reduction. | `src/sim/world/mod.rs:489-502`, `src/sim/world/world_orders.rs:390` |
| Growth model | `OreGrowthState` stores scan cursor and reservoir candidates; `tick_ore_growth` scans `resource_nodes` and executes growth/spread after a full scan wraps. | `src/sim/ore_growth.rs:120-156`, `src/sim/ore_growth.rs:171-260` |
| Growth/spread overlay writes | Ore growth uses `set_overlay_data`; spread uses `place_overlay` from the source overlay id. | `src/sim/ore_growth.rs:229-233`, `src/sim/ore_growth.rs:292-332` |
| TIBTRE integration | Terrain spawners independently add ore stock and mutate `OverlayGrid`, after `tick_ore_growth` in Phase 7. | `src/sim/terrain_spawn.rs:81-88`, `src/sim/terrain_spawn.rs:193-227`, `src/sim/world/mod.rs:1546-1563` |
| Tick order | Combat/smudge drains before ore growth; production/repairs/docks run before ore growth; TIBTRE runs after ore growth. | `src/sim/world/mod.rs:1338-1563` |
| Hash coverage | `state_hash` hashes `resource_nodes`, `terrain_spawners`, `default_ore_overlay_id`, and occupied overlay cells. | `src/sim/world/world_hash.rs:173-186`, `src/sim/world/world_hash.rs:276-286` |
| Hash gap | `OreGrowthState` and `OreGrowthConfig` are serialized through `ProductionState`, but no `ore_growth_state` or `ore_growth_config` symbol appears in `world_hash.rs`. | `src/sim/production/production_types.rs:196-210`, `src/sim/world/world_hash.rs:173-186` |
| Serialization shape | `Simulation`, `ProductionState`, `OreGrowthState`, and `OverlayGrid` derive serde; `OverlayGrid::dirty_cells` is skipped. | `src/sim/world/mod.rs:256-263`, `src/sim/production/production_types.rs:196-210`, `src/sim/ore_growth.rs:119-135`, `src/sim/overlay_grid.rs:39-48` |

## 3. Current Data Model Mismatch

The live bug comes from using `remaining` as both the economic stock and the effective density level source. `seed_resource_nodes_from_overlays` stores `OverlayData + 1` density levels, while gamemd `Reduce_Tiberium` returns the current `OverlayData` value on full removal for the verified `OverlayData=11` case. Current Rust therefore cannot reproduce the verified `11` return from a real map-seeded frame-11 cell without either changing the resource model or making reduction consult the overlay byte.

The growth side is a larger mismatch. `ore_growth.rs` describes and implements a RA1-style full-map scan plus reservoir candidates. Existing RA2/YR docs describe per-tiberium growth/spread queues with per-type timers, bitmaps, heap entries, and depletion-time spread reseed. The current Rust state has nowhere to represent per-type queue membership or the "cell-in-queue" bitmap that `Reduce_Tiberium` side effects need to clear/reinsert.

## 4. Integration Points And Blast Radius

| Surface | Why it is touched | Ownership risk |
|---|---|---|
| `src/sim/miner/mod.rs` | `ResourceNode`, `ResourceType`, and current combat-facing `reduce_tiberium` live here. | This module is becoming too broad if it owns global cell/tiberium mutation; prefer moving authoritative cell ore reduction into a dedicated sim module. |
| `src/sim/miner/miner_system.rs` | `handle_harvest` and `extract_bales_max` are the active CMIN/HARV harvest path. | Must stop bypassing the authoritative reduction helper; keep miner state transitions separate from cell mutation details. |
| `src/sim/production/production_queue.rs` | Seeds resource nodes from map overlays and currently creates the 12-level frame-11 stock. | Needs real-overlay acceptance tests; changing seeding alone may break growth/spawn assumptions if no helper owns overlay-data semantics. |
| `src/sim/ore_growth.rs` | Current scan/reservoir model lacks RA2/YR queues and depletion-time reseed. | Likely replacement or major adaptation; queue state must be deterministic, serialized, and hashed. |
| `src/sim/overlay_grid.rs` | Owns overlay byte mutation and dirty-cell tracking. | Existing dirty-cell drain is app-timed; an authoritative sim reduction helper may need synchronous terrain recalc access or a sim-visible dirty result. |
| `src/sim/world/mod.rs` | Owns tick order and `Simulation` fields/events. | Any new queue state or dirty event likely belongs under `Simulation::production` or a sibling sim-owned tiberium state; avoid render/ui/audio dependencies. |
| `src/sim/world/world_hash.rs` | Deterministic hash currently omits `OreGrowthState`. | New RA2/YR queue state must be hashed, and current omission should be fixed if growth remains deterministic gameplay state. |
| `src/sim/combat/*` and `src/sim/smudge_grid.rs` | Combat/smudge paths reduce ore via the old helper and currently cannot update overlay/grid/queue side effects. | Authoritative reduction API must serve both miner harvest and combat/smudge damage, or the engine will keep split parity. |
| `src/sim/terrain_spawn.rs` | Adds ore stock/overlay independently after ore growth. | If RA2/YR queue state exists, TIBTRE/PlaceTiberium decisions must specify whether new ore enters queues immediately, deferred, or via rebuild per verified docs. |
| `src/app_sim_tick.rs` | Drains overlay dirtiness and syncs new overlay entries to app render state. | Avoid moving render concerns into sim; but passability/minimap dirty consequences need an explicit sim/app boundary. |

## 5. Tick Order Notes

Current `advance_tick` order relevant to ore:

1. Combat receives `&mut self.production.resource_nodes` plus read-only `overlay_grid` and may reduce ore through combat logic.
2. Smudge dispatch drains pending requests before ore growth and also mutates `resource_nodes` through `reduce_tiberium`.
3. Phase 7 runs production, repairs, building docks, aircraft docks.
4. `tick_ore_growth` reads/writes `resource_nodes`, `OreGrowthState`, optional `PathGrid`, optional mutable `OverlayGrid`, and RNG.
5. `tick_terrain_spawners` runs after ore growth and may add ore/overlay changes.
6. End of `advance_tick` computes `state_hash`.
7. App code later drains `OverlayGrid::dirty_cells` and recalculates overlay passability.

Implication: if a full reduction must make passability/terrain visible before later sim decisions, draining dirty cells only after `advance_tick` is too late. If the intended parity only requires app-render/minimap dirtying after the sim tick, the current boundary may be acceptable, but the contract's "before next sim decision" test needs a sim-owned recalc or a documented tick-order adjustment.

## 6. Hash / Serialization Concerns

- `OreGrowthState` is serde state and affects future ore placement decisions, but it is not currently included in `Simulation::state_hash`. This is a deterministic replay/desync risk.
- `OverlayGrid::dirty_cells` is explicitly skipped and intended to be empty at tick boundaries. New persistent queue bitmaps/heaps must not follow that pattern; they are gameplay state.
- `radar_terrain_dirty_cells` and generation are skipped runtime/render invalidation state on `Simulation`; they should not substitute for deterministic tiberium queue state.
- New queue containers should use deterministic ordering or explicit heap ordering that serializes and hashes exactly. Avoid `HashMap`/nondeterministic iteration.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ProductionState::resource_nodes` ownership | verified | `production_types.rs:197-210` | none |
| Map overlay seeding | verified | `production_queue.rs:132-170` | Need implementation test with real overlay-backed cell. |
| Miner harvest call path | verified | `miner_system.rs:520-534`, `miner_system.rs:804-855` | Exact future API shape deferred. |
| Combat/smudge reduction call sites | verified | `smudge_dispatch.rs:140/226/290`, `combat/mod.rs:1116-1126` | Need later behavior proof for overlay side effects in combat contexts. |
| OverlayGrid mutation/dirty semantics | verified | `overlay_grid.rs:40-48`, `overlay_grid.rs:92-118`, `app_sim_tick.rs:679-692` | Whether passability must recalc inside sim remains behavior-policy follow-up. |
| Current ore growth model | verified | `ore_growth.rs:1-15`, `ore_growth.rs:120-260` | RA2/YR replacement design deferred to parent swarm/brainstorm. |
| Terrain spawner integration | verified | `terrain_spawn.rs:81-88`, `terrain_spawn.rs:193-227`, `world/mod.rs:1554-1563` | Queue interaction for TIBTRE remains dependent on binary slot reports. |
| World hash coverage | verified | `world_hash.rs:173-186`, `world_hash.rs:276-286` | Add hash coverage for any new queue state; consider current `OreGrowthState` omission. |
| Serialization | verified | serde derives in listed files | Save compatibility migration strategy not investigated. |
| Exact RA2/YR queue binary behavior | deferred | existing `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md` | Covered by other reswarm slots, not this Rust-only slot. |

## 8. Open Questions - Final State

- `[RESOLVED] RUST-01 - Where is authoritative ore stock stored? -> In ProductionState::resource_nodes as BTreeMap keyed by cell.` (evidence: `src/sim/production/production_types.rs:197-204`)
- `[RESOLVED] RUST-02 - Does miner harvest call reduce_tiberium? -> No; it calls extract_bales_max.` (evidence: `src/sim/miner/miner_system.rs:520-534`)
- `[RESOLVED] RUST-03 - What does extract_bales_max mutate? -> resource_nodes and optional OverlayGrid only.` (evidence: `src/sim/miner/miner_system.rs:804-855`)
- `[RESOLVED] RUST-04 - Where does combat ore damage enter? -> combat AoE and smudge dispatch call miner::reduce_tiberium.` (evidence: `src/sim/combat/mod.rs:1116-1126`, `src/sim/combat/smudge_dispatch.rs:140`)
- `[RESOLVED] RUST-05 - Does reduce_tiberium update overlay bytes? -> No; signature only accepts resource_nodes, cell, amount.` (evidence: `src/sim/miner/mod.rs:395-399`)
- `[RESOLVED] RUST-06 - How do overlay mutations notify later systems? -> OverlayGrid pushes dirty_cells; app drains them after advance_tick.` (evidence: `src/sim/overlay_grid.rs:92-118`, `src/app_sim_tick.rs:679-692`)
- `[RESOLVED] RUST-07 - Is OreGrowthState RA2/YR queue-shaped? -> No; it is scan cursor plus reservoir candidate vectors.` (evidence: `src/sim/ore_growth.rs:120-135`)
- `[RESOLVED] RUST-08 - Is OreGrowthState serialized? -> Yes, through ProductionState serde derives.` (evidence: `src/sim/production/production_types.rs:196-210`)
- `[RESOLVED] RUST-09 - Is OreGrowthState hashed? -> No direct hash reference found in world_hash.rs; production hash includes resources/spawners/default overlay id but not ore_growth_state/config.` (evidence: `src/sim/world/world_hash.rs:173-186`)
- `[RESOLVED] RUST-10 - Where does TIBTRE run relative to ore growth? -> After tick_ore_growth in Phase 7.` (evidence: `src/sim/world/mod.rs:1546-1563`)
- `[RESOLVED] RUST-11 - Does sim already have radar dirty state? -> Yes, mark_radar_terrain_dirty_cells exists and is used by bridge repair, not ore reduction.` (evidence: `src/sim/world/mod.rs:489-502`, `src/sim/world/world_orders.rs:390`)
- `[DEFERRED] RUST-12 - Should passability recalc move into sim reduction?` (category: `requires-different-system-context`; reason: needs parent reconciliation with binary same-call `RecalcAttributes` and current app-owned PathGrid rebuild flow; next-step-if-pursued: design sim/app dirty boundary before coding)
- `[DEFERRED] RUST-13 - Exact save migration for replacing OreGrowthState.` (category: `out-of-scope`; reason: this slot is ownership mapping only; next-step-if-pursued: plan serialization/versioning after architecture choice)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Full ore reduction must be one authoritative mutation visible to harvest, combat, and smudge callers. | Rust split: `extract_bales_max` at `miner_system.rs:804-855`; `reduce_tiberium` at `miner/mod.rs:395-424`; call sites in `combat/mod.rs:1116-1126` and `smudge_dispatch.rs:140/226/290`. | Split implementation; miner bypasses helper, helper lacks overlay/queue side effects. | New or moved sim tiberium/cell-resource module; `miner_system.rs`; `combat/*`; `overlay_grid.rs`; `world/mod.rs`. | One helper returns removed amount and applies resource node, overlay byte, dirty/passability/radar/queue effects in a deterministic order. | Harvest `OverlayData=11` real cell and crater-damage ore both pass through same helper; proposed test `reduce_tiberium_authoritative_path_shared_by_harvest_and_crater`. | Do not keep a miner-only fix plus a stale combat helper; that preserves split parity bugs. |
| RA2/YR queue state is gameplay state, not render dirtiness. | Existing RA2/YR docs: `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md`; current Rust state shape `ore_growth.rs:120-260`; hash omission `world_hash.rs:173-186`. | Current scan/reservoir model lacks queue bitmaps/heaps and is not hashed. | `ore_growth.rs` replacement/adaptation; `ProductionState`; `world_hash.rs`; serde state. | Add deterministic per-type queue state with serialization and hash coverage, or explicitly replace `OreGrowthState` with a queue-backed model. | Two sims differing only in tiberium queue membership hash differently and diverge predictably on growth tick; proposed test `tiberium_queue_state_changes_world_hash`. | Do not store queue membership in `OverlayGrid::dirty_cells` or any skipped/render-only state. |
| Full removal dirties overlay/passability/radar surfaces before stale terrain can drive later decisions. | Rust overlay dirty: `overlay_grid.rs:92-118`; app drain: `app_sim_tick.rs:679-692`; radar dirty method: `world/mod.rs:489-502`. | Full removal only queues overlay dirty; no radar dirty; passability recalc is app-side after advance_tick. | Authoritative reduction result type; `Simulation::mark_radar_terrain_dirty_cells`; path/passability invalidation boundary; app drain path. | Full reduction should publish a deterministic dirty result that the world/app boundary can apply in correct order without sim depending on render/ui/audio. | Empty tiberium cell, immediately query overlay and movement/passability before next high-level frame; proposed test `reduce_tiberium_full_removal_recalcs_or_queues_passability_before_next_decision`. | Do not call render/minimap/sidebar/audio from sim; use sim-owned dirty events/state. |

## 10. Negative Facts / Do Not Do

- Do not fix only `seed_resource_nodes_from_overlays`; the same authoritative semantics must serve harvest, combat, smudge, growth, and terrain-spawn interactions.
- Do not make `src/sim/miner/mod.rs` the long-term owner of global cell tiberium behavior just because `ResourceNode` currently lives there.
- Do not leave new RA2/YR tiberium queue state out of `state_hash`; current `OreGrowthState` omission is already a warning.
- Do not use `OverlayGrid::dirty_cells` for persistent gameplay queue membership; it is serde-skipped and intended to drain at tick/app boundaries.
- Do not let `sim/` depend on render, UI, sidebar, audio, or net to model dirty tactical/radar side effects.

## 11. Remaining Uncertainty

- Exact API boundary for synchronous `RecalcAttributes` parity is unresolved: either sim owns enough terrain/passability state to update immediately, or it emits a deterministic dirty result applied before any dependent sim query.
- Save compatibility migration is not designed if `OreGrowthState` is replaced with RA2/YR queue state.
- Other reswarm slots must settle exact RA2/YR queue insertion/removal rules before coding the queue model.

## 12. Stale Docs / Replacement Wording

Stale wording found in `src/sim/ore_growth.rs:1-15` comments:

> Ports the proven RA1 algorithm ... matching RA1 MapClass::Logic.

Replacement wording for future implementation:

> Current transitional ore growth uses an RA1-derived scan/reservoir model. Standard RA2/YR tiberium parity requires per-tiberium growth/spread queue state with deterministic queue membership, hashing, serialization, and `Reduce_Tiberium` full-removal reseed integration.

Also replace any phrase claiming `miner::reduce_tiberium` mirrors `CellClass::Reduce_Tiberium` with:

> Transitional resource-node-only reduction helper; does not yet model full RA2/YR `CellClass::Reduce_Tiberium` overlay, terrain/radar dirty, or tiberium queue side effects.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game/docs/contracts/2026-05-23-chrono-miner-reduce-tiberium-implementation-contract.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/CMIN_HARVEST_DENSITY_CARGO_REDUCE_TIBERIUM_TRACE.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_types.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_queue.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/ore_growth.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/overlay_grid.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/terrain_spawn.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/smudge_dispatch.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_hash.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs`
