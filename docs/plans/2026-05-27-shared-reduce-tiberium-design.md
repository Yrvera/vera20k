# Shared Reduce_Tiberium Design

## Goal

Implement one shared sim-owned `CellClass::Reduce_Tiberium` equivalent for all active YR callers so harvest, crater/smudge, and existing combat ore-damage paths mutate tiberium cells, overlay state, dirty outputs, and queue side effects through the same parity boundary.

## Architecture Context

Current Rust has three competing tiberium mutation paths:

- Miner harvest uses `extract_bales_max` in `src/sim/miner/miner_system.rs`, directly deriving extracted bales from `ResourceNode.remaining` and clearing `OverlayGrid` only on full drain.
- Combat/smudge paths call `sim::miner::reduce_tiberium` from `src/sim/combat/smudge_dispatch.rs` and `src/sim/combat/mod.rs`; that helper only mutates `resource_nodes`.
- Ore growth/spread lives in `src/sim/ore_growth.rs`. It has native-style growth queue insertion for TIBTRE placement, but the main tick path is still a scan/reservoir model and does not expose the per-type spread queue reseed that `Reduce_Tiberium` full removal requires.

The correct owner is not `sim::miner`. In gamemd, harvesters, crater animations, and weapon cell-side effects reach the shared cell mutation primitive `CellClass::Reduce_Tiberium`. In Rust this belongs under a new shared sim module, tentatively `src/sim/tiberium/`, with miner, combat, smudge, terrain spawning, and ore growth as callers.

The module must stay inside `sim/` and may publish deterministic dirty events, but it must not depend on render, UI, sidebar, audio, or net. App/render layers already drain overlay dirty cells and radar dirty cells above sim.

## Impact Analysis

Touched modules:

- `src/sim/tiberium/` new shared owner for reduction, tiberium cell metadata helpers, and reduction outcomes.
- `src/sim/miner/miner_system.rs` harvest extraction rewired from `extract_bales_max` to shared reduction.
- `src/sim/miner/mod.rs` old miner-owned `reduce_tiberium` removed or replaced by a compatibility wrapper during migration.
- `src/sim/combat/smudge_dispatch.rs` crater reduction rewired while preserving reduction-before-smudge order.
- `src/sim/combat/mod.rs` existing weapon ore-damage caller rewired to shared reduction without changing its currently requested amount/gates in this patch.
- `src/sim/ore_growth.rs` gains the deterministic per-type spread queue surface needed by full-removal reseed.
- `src/sim/overlay_grid.rs` remains the overlay mutation and dirty-cell surface; shared reduction calls it rather than duplicating overlay storage.
- `src/sim/world/mod.rs` may need small call-site changes so combat/smudge reduction receives overlay/growth/dirty context.
- `src/sim/world/world_hash.rs` must hash any new persistent queue state.

Risk areas:

- Sim tick ordering: crater reduction must still occur before smudge placement and before ore growth reads resource density.
- Determinism: new spread queue state and any priority values must be serialized/hashed; no unordered maps for queue iteration.
- Architecture: avoid leaking render concerns into sim. `TacticalClass::DirtyScreenRect` should become a sim dirty-output event, not a render call.
- Existing tests that hand-seed `ResourceNode.remaining` may bypass live `OverlayData`; new tests must use real overlay-backed cells.

## Chosen Approach

Approach A: add a shared `sim::tiberium` module and make it the authoritative tiberium cell mutation boundary.

The central API should be shaped around a context and result:

```rust
pub struct ReduceTiberiumContext<'a> {
    pub resource_nodes: &'a mut BTreeMap<(u16, u16), ResourceNode>,
    pub overlay_grid: Option<&'a mut OverlayGrid>,
    pub ore_growth_state: &'a mut OreGrowthState,
    pub radar_dirty_cells: &'a mut Vec<(u16, u16)>,
    pub tactical_dirty_cells: &'a mut Vec<(u16, u16)>,
}

pub struct ReduceTiberiumOutcome {
    pub removed_amount: u16,
    pub resource_type: Option<ResourceType>,
    pub fully_removed: bool,
    pub dirty_cells: Vec<(u16, u16)>,
}
```

Exact field names may differ, but the API must make the side effects explicit and impossible to forget at call sites.

`reduce_tiberium(ctx, cell, amount)` should:

1. Return zero for amount zero, missing tiberium, or empty cell.
2. Read the authoritative overlay data byte when an overlay cell is present.
3. For partial removal, subtract the requested amount from `OverlayData`, sync `ResourceNode`, mark overlay/tactical dirty, and return requested amount.
4. For full removal, clear overlay type/data, remove or zero the `ResourceNode`, publish same-call dirty outputs, clear queue membership for the removed cell, reseed eligible 8-neighbors into the removed type's spread queue, and return the pre-removal density byte according to the verified gamemd semantics.

The design deliberately does not change the unresolved weapon AoE gate/amount in this implementation slice. It only routes the existing Rust request through the shared side-effect boundary. A separate `/re-investigate` remains needed for exact weapon gate/amount parity.

## Tiny-Detail Ledger

- Active standard YR callers include harvester `Harvest_Ore_Tick`, crater animation `Reduce_Tiberium(6)`, and weapon AoE cell-side reduction. Source: `COMBAT_SMUDGE_REDUCE_TIBERIUM_SIDE_EFFECTS_TRACE.md`; `HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md`.
- The owner is shared `CellClass::Reduce_Tiberium`, not miner-specific code. Source: `COMBAT_SMUDGE_REDUCE_TIBERIUM_SIDE_EFFECTS_TRACE.md` stage "Ownership boundary".
- Partial reduction subtracts from `CellClass+0x11E OverlayData` and returns the requested amount. Source: `COMBAT_SMUDGE_REDUCE_TIBERIUM_SIDE_EFFECTS_TRACE.md` stage "Partial overlay mutation".
- Full removal writes overlay type none and overlay data zero before recalculation/dirty side effects. Source: `CMIN_LIFECYCLE_ORE_DEPLETION_SHORT_SCAN_RETARGET_TRACE_2026-05-27.md` stage 3.
- Full removal returns the pre-removal current density byte; verified max-density `OverlayData=11` returns `11`, not `12`. Source: `HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md` section 3.6 and handoff.
- Harvest cargo and later deposit value use exactly the returned removed amount. Source: `HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md` section 3.6.
- Full removal calls `RecalcAttributes`, marks radar terrain dirty, clears spread bitmap entries, reseeds neighbors, and dirties tactical screen in the same reduction call. Source: `CMIN_LIFECYCLE_ORE_DEPLETION_SHORT_SCAN_RETARGET_TRACE_2026-05-27.md` stage 3.
- Full-removal spread reseed visits the 8 neighbors in gamemd direction-table order and enqueues valid neighbors for the removed tiberium type when allowed. Source: `CMIN_ORE_DEPLETION_RETARGET_SHORT_SCAN_TRACE.md` stage 5.
- Density-11 growth-queue detour is a net no-op for full-removal `OverlayData=11` because the callee sees the pre-decrement density and refuses enqueue. Source: `CMIN_LIFECYCLE_MAX_DENSITY_RIPARIUS_HARVEST_TRACE_2026-05-27.md` stage 6.
- Crater ore reduction amount is six and occurs before smudge/debris placement, even if smudge placement later fails. Source: `COMBAT_SMUDGE_REDUCE_TIBERIUM_SIDE_EFFECTS_TRACE.md` stages "Crater reduction amount" and "Crater reduction ordering".
- Weapon AoE exact gate/amount is not proven enough for a gate rewrite in this design. Source: `COMBAT_SMUDGE_REDUCE_TIBERIUM_SIDE_EFFECTS_TRACE.md` stages "Weapon reduction gate" and "Weapon reduction amount".
- Dirty outputs must stay deterministic sim state/events, not direct render/UI calls. Source: project `AGENTS.md` architecture boundary and existing `Simulation::mark_radar_terrain_dirty_cells`.

## Design

### Components

`sim::tiberium::reduction`

- Owns the shared `reduce_tiberium` function and `ReduceTiberiumOutcome`.
- Converts live overlay data plus resource type into removed amount and remaining density.
- Keeps `ResourceNode` as Rust's compatibility/resource cache, not as the authoritative density source when overlay data is available.

`sim::tiberium::queues` or additions to `ore_growth.rs`

- Adds per-type spread queue insertion and cell membership tracking needed by depletion-time reseed.
- Preserves existing native growth queue priority behavior already used by TIBTRE placement.
- Hashes/serializes new persistent queue fields.

`sim::tiberium::dirty`

- Small data types for tactical/radar/terrain dirty outputs, or simple vectors on the reduction context if no extra module is needed.
- Uses existing `OverlayGrid` dirty-cell flow for overlay passability recalculation.
- Uses existing `Simulation::mark_radar_terrain_dirty_cells` style for minimap refresh.

### Interfaces / Contracts

Reduction callers must pass enough context for side effects. A resource-only reduction function is not allowed for active gameplay callers.

Miner harvest:

- Computes remaining capacity as gamemd request amount.
- Calls shared reduction.
- Appends `removed_amount` cargo units using the returned resource type/value.
- Full cell removal and partial density updates come from shared reduction.

Crater/smudge:

- Calls shared reduction with amount 6 before smudge placement.
- Does not inspect cargo/value fields.
- Preserves existing order of pending smudge request drain.

Combat weapon ore damage:

- Keeps current caller-computed amount for now.
- Calls shared reduction so overlay/queue/dirty side effects are no longer missing.
- Exact gate/amount remains a known blocker outside this design slice.

Ore growth/spread:

- Exposes `reseed_spread_neighbors_after_reduction(cell, removed_type, overlay_grid/resolved validation)` or equivalent through `OreGrowthState`.
- New queue state must be deterministic, serialized, and hashed.

### Data Flow

1. Caller determines the target cell and requested amount.
2. Caller builds a reduction context from simulation-owned state.
3. `sim::tiberium::reduce_tiberium` reads overlay/resource state and classifies partial vs full.
4. Partial path updates overlay data and resource cache, emits dirty state, returns requested amount.
5. Full path clears overlay/resource state, emits dirty state, clears/reseeds queue membership, returns pre-removal density.
6. Caller applies caller-specific downstream behavior: cargo insertion, smudge placement, or no extra work.
7. App-level dirty drain later refreshes passability/render surfaces without sim depending on render.

### Error Handling

This is deterministic sim logic, so missing optional grids should not panic in headless tests. The shared function should return zero/no-op only for truly missing tiberium; lack of optional app/render surfaces should still update resource/overlay state when available and expose what was skipped through tests or explicit context type choices.

For production gameplay, active callers should prefer full context. Tests may use a minimal context only for pure return-value checks.

### Testing Strategy

Required focused tests:

- `reduce_tiberium_partial_updates_overlay_data`: start overlay data 8, reduce 2, assert return 2, overlay data 6, resource cache synced, dirty emitted.
- `reduce_tiberium_full_removal_overlaydata_11_returns_11`: start real overlay data 11, reduce 20, assert return 11, overlay cleared, resource node removed, dirty emitted.
- `cmin_overlaydata_11_extracts_11_bales`: real overlay-backed CMIN harvest adds 11 ore bales worth 275, not 12 worth 300.
- `cmin_gem_overlaydata_11_extracts_11_gem_units`: same for Cruentus/GEM, value 550.
- `reduce_tiberium_full_removal_reseeds_spread_neighbors`: with valid neighbors and spread enabled, full removal enqueues eligible neighbors in deterministic order.
- `crater_reduce_tiberium_6_before_smudge_with_side_effects`: crater path reduces ore before smudge placement and uses shared dirty/overlay behavior.
- `combat_ore_damage_uses_shared_reduction`: existing weapon ore-damage request reaches shared reduction and no longer imports `sim::miner::reduce_tiberium`.
- State hash test covering new spread queue membership/state.

Broader regression tests:

- Existing miner lifecycle tests around return/deposit still pass after cargo amount changes.
- Existing TIBTRE growth queue tests still pass.
- Existing smudge placement ordering tests still pass.

## Architectural Decisions

- Add `sim::tiberium` instead of expanding `sim::miner`, because the behavior is cell/resource ownership shared by harvest, combat, smudge, terrain, and growth.
- Keep `OverlayGrid` as the overlay storage owner; shared reduction calls its mutation methods instead of duplicating overlay state.
- Keep dirty outputs as sim-owned events and existing app dirty drains; no direct render/UI calls from sim.
- Do not rewrite weapon AoE gates in this patch. The shared side-effect boundary is implementation-safe; exact weapon reduction amount/gates need more RE.
- Do not preserve the RA1 scan/reservoir ore-growth path as a parity claim. Any fallback must be explicitly transitional and not used to justify parity.

## Alternatives Considered

### B. Expand `sim::miner::reduce_tiberium`

Rejected. It is faster to patch, but keeps the wrong ownership boundary and continues to make combat/smudge depend on a miner helper for a shared cell primitive.

### C. Rewrite all YR tiberium growth/spread first

Rejected for this slice. It is likely the long-term endpoint, but the shared reduction boundary can be implemented first while adding only the queue state needed for full-removal reseed. Exact natural growth/spread processing can continue as a later, separately scoped parity patch.

### D. Patch CMIN only

Rejected. It would fix one visible economic bug but leave crater/smudge and combat paths with known overlay/dirty/queue drift.
