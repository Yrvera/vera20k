# Shared Reduce_Tiberium Implementation Plan

Date: 2026-05-27
Status: READY_FOR_IMPLEMENTATION
Design: `docs/plans/2026-05-27-shared-reduce-tiberium-design.md`
Contracts: `docs/contracts/2026-05-23-chrono-miner-reduce-tiberium-implementation-contract.md`, `docs/contracts/2026-05-24-yr-ore-tiberium-boundary-implementation-contract.md`
Scope: Rust implementation plan only; no gameplay code changes in this document.

## Goal

Replace miner-owned and resource-only tiberium reduction with one shared sim-owned `Reduce_Tiberium` boundary used by miner harvest, crater/smudge reduction, and the existing combat ore-damage caller.

The first implementation slice must close the verified shared side-effect gap:

- returned removal amount follows live `OverlayData` semantics;
- partial reduction updates overlay data and resource cache;
- full removal clears overlay/resource state;
- full removal emits deterministic terrain/radar/tactical dirty outputs;
- full removal reseeds spread neighbors through persistent ore-growth state;
- active callers no longer import or call `sim::miner::reduce_tiberium`.

## Current Code Touchpoints

- `src/sim/miner/miner_system.rs`
  - `extract_bales_max` currently derives extraction from `ResourceNode.remaining / base`.
  - Full drain removes `resource_nodes` and calls `OverlayGrid::clear_overlay`.
  - `handle_harvest` builds cargo from returned `CargoBale` values.
- `src/sim/miner/mod.rs`
  - Defines `ResourceType`, `ResourceNode`, `CargoBale`, and the old `reduce_tiberium` helper.
  - The old helper is resource-only and should stop being the active caller API.
- `src/sim/combat/smudge_dispatch.rs`
  - Crater paths call `reduce_tiberium(..., 6)` before smudge placement.
  - This ordering must remain unchanged.
- `src/sim/combat/mod.rs`
  - `destroy_ore_at_impact` computes an existing `ore_damage` and calls the old helper.
  - Exact weapon gates/amounts remain out of scope; only the mutation boundary changes.
- `src/sim/ore_growth.rs`
  - Has native-style growth queue entry priority for TIBTRE placement.
  - Does not yet expose a per-type spread queue or depletion-time reseed.
- `src/sim/overlay_grid.rs`
  - Owns overlay mutation and dirty cells.
  - `clear_overlay`, `place_overlay`, and `set_overlay_data` should remain the mutation surface.
- `src/sim/world/mod.rs`
  - Owns full simulation state needed to build reduction context.
  - Combat/smudge call sites may need to pass overlay/growth/dirty context instead of only `resource_nodes`.
- `src/sim/world/world_hash.rs`
  - Must hash new persistent queue state.
- `src/app_sim_tick.rs`
  - Drains overlay dirty cells for passability/render sync. This remains the app boundary; sim must not call render.

## Implementation Shape

### 1. Add `sim::tiberium`

Create a new module:

```text
src/sim/tiberium/mod.rs
```

Initial responsibilities:

- define `ReduceTiberiumContext`;
- define `ReduceTiberiumOutcome`;
- define `reduce_tiberium(ctx, cell, amount)`;
- provide tiny helper functions for density/value conversion.

Recommended context shape:

```text
ReduceTiberiumContext
  resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>
  overlay_grid: Option<&mut OverlayGrid>
  ore_growth_state: &mut OreGrowthState
  radar_dirty_cells: &mut Vec<(u16, u16)>
  tactical_dirty_cells: &mut Vec<(u16, u16)>
```

Keep the first implementation practical: if `tactical_dirty_cells` does not yet have a dedicated `Simulation` field, introduce a small sim-owned dirty vector or fold the tactical dirty cells into a clearly named reduction outcome consumed by tests. Do not call render/UI from sim.

### 2. Implement OverlayData-First Reduction Semantics

`reduce_tiberium` should read density in this priority order:

1. If `overlay_grid` has an overlay at the cell, use `overlay_data` as the gamemd current density byte.
2. If no overlay grid is available, fall back to existing `ResourceNode.remaining / base` for headless tests only.

For partial reduction:

- branch when `amount < current_density_threshold` following verified semantics;
- subtract amount from `overlay_data`;
- sync `ResourceNode.remaining` to the same effective density;
- push overlay dirty via `OverlayGrid::set_overlay_data`;
- mark radar/tactical dirty outputs;
- return `removed_amount = amount`.

For full removal:

- return pre-removal density amount as verified, including `OverlayData=11 -> 11`;
- clear overlay type/data through `OverlayGrid::clear_overlay`;
- remove the `ResourceNode`;
- mark radar/tactical dirty outputs;
- trigger ore-growth spread membership clear/reseed;
- do not consume RNG unless the verified queue operation requires it.

The exact density branch is subtle. Follow the existing verified tests from the contracts; if a direct reading of old traces conflicts, stop and resolve before coding more.

### 3. Add Persistent Spread Queue Reseed Surface

Extend `OreGrowthState` with the smallest persistent surface needed by full-removal reseed:

- per-resource-type spread queue entries;
- per-resource-type membership sets or deterministic equivalent to prevent duplicate queue entries;
- clear membership for the removed cell across all types;
- visit 8 neighbors in gamemd direction-table order and enqueue eligible neighbors for the removed resource type.

Use `BTreeMap`/`BTreeSet` or sorted `Vec`s for deterministic iteration and hashing.

Do not replace the entire natural growth/spread tick in this patch unless required by compilation. Existing scan/reservoir tick may remain as a known non-parity path, but depletion-time reseed must write into the new persistent queue surface and be covered by tests.

### 4. Hash And Serialize New Queue State

Update:

- `OreGrowthState` serde fields;
- `OreGrowthState::hash_state`;
- any snapshot default handling if old saves need defaults.

Hash:

- spread queue cells in deterministic order;
- membership state;
- resource type discriminants;
- any priority/timing fields added.

### 5. Rewire Miner Harvest

In `miner_system.rs`:

- replace `extract_bales_max` use in `handle_harvest` with shared `sim::tiberium::reduce_tiberium`;
- request amount remains empty capacity in tiberium units;
- append `removed_amount` cargo entries using returned resource type and config value;
- preserve the existing full/empty/short-scan return flow around the extraction result;
- update or retire `extract_bales_max` tests to call shared reduction where appropriate.

Important: real overlay-backed cells must drive the main tests. Hand-seeded `ResourceNode { remaining: 12 * base }` is not enough to prove parity.

### 6. Rewire Crater/Smudge

In `smudge_dispatch.rs`:

- replace old helper calls with shared reduction;
- preserve amount `6`;
- preserve reduction-before-smudge placement order;
- pass enough context into `drain_smudge_spawn_requests` from `Simulation::advance_tick`.

This likely requires changing function signatures that currently accept only `resource_nodes`.

### 7. Rewire Existing Combat Ore Damage

In `combat/mod.rs`:

- replace old helper call in `destroy_ore_at_impact`;
- pass context from caller if practical, or return reduction requests from combat and apply them in `Simulation::advance_tick` where full context exists.

Preferred ownership:

- combat can compute target cells and requested amount;
- simulation applies shared tiberium reduction with full context.

If passing full context into combat creates borrow conflicts, introduce a small deterministic `TiberiumReductionRequest { cell, amount }` list in combat result and drain it immediately after combat, before smudge and ore growth.

Do not change the current weapon amount/gates in this patch. Add a TODO pointing to the blocked RE item if needed.

### 8. Retire Miner-Owned Helper From Active Callers

After rewiring:

- remove `sim::miner::reduce_tiberium`, or keep it only as a `#[cfg(test)]` compatibility shim calling the shared helper with minimal context;
- update imports so combat/smudge no longer depend on `sim::miner::reduce_tiberium`;
- add a grep/compile-level check by test or review: no active caller imports the old helper.

## Test Plan

Add or update tests in the closest existing modules:

### Shared Reduction Tests

Likely location: new `src/sim/tiberium/mod.rs` tests.

1. `reduce_tiberium_partial_updates_overlay_data`
   - Setup overlay-backed ore cell with `overlay_data=8`.
   - Reduce by `2`.
   - Assert return `2`, overlay data becomes `6`, resource cache syncs, dirty outputs include the cell.

2. `reduce_tiberium_full_removal_overlaydata_11_returns_11`
   - Setup overlay-backed Riparius cell with `overlay_data=11`.
   - Reduce by `20`.
   - Assert return `11`, overlay cleared, resource node removed, radar/tactical dirty output emitted.

3. `reduce_tiberium_full_removal_reseeds_spread_neighbors`
   - Setup removed cell with valid neighboring candidates.
   - Reduce to full removal.
   - Assert spread queue/membership receives eligible neighbors for the removed resource type in deterministic order.

4. `reduce_tiberium_zero_or_missing_cell_noops`
   - Assert amount zero and missing/non-tiberium cells return zero and emit no dirty/queue output.

### Miner Tests

Likely location: `src/sim/miner/miner_tests.rs`.

5. `cmin_overlaydata_11_extracts_11_bales`
   - Real overlay-backed ore cell with `overlay_data=11`, empty CMIN, stock config.
   - Run extraction gate.
   - Assert cargo len `11`, cargo value `275`, overlay cleared, node removed.

6. `cmin_gem_overlaydata_11_extracts_11_gem_units`
   - Real overlay-backed GEM/Cruentus cell with `overlay_data=11`.
   - Assert cargo len `11`, cargo value `550`.

7. `harvest_partial_reduction_updates_overlay_before_next_scan`
   - Partial harvest leaves overlay data lowered and resource cache synced before any next miner scan/path query.

### Combat/Smudge Tests

Likely locations: `src/sim/combat/combat_tests.rs`, `src/sim/combat/smudge_dispatch.rs` tests.

8. `crater_reduce_tiberium_6_before_smudge_with_side_effects`
   - Crater anim on ore.
   - Assert reduction happens before smudge placement and overlay dirty/side effects are present.

9. `combat_ore_damage_uses_shared_reduction`
   - Existing weapon ore-damage request reaches shared helper outcome.
   - Assert old miner helper is not imported by combat/smudge code.

### Hash/Serialization Tests

10. `ore_growth_spread_queue_state_hashes`
   - Mutate spread queue/membership.
   - Assert world hash changes.

11. Existing TIBTRE growth queue priority tests still pass.

## Implementation Order

1. Create `sim::tiberium` module with pure/minimal context tests for return-value semantics.
2. Add `OreGrowthState` spread queue/membership APIs and hash coverage.
3. Add full-context reduction side effects: overlay mutation, resource sync/removal, dirty outputs, spread reseed.
4. Rewire miner harvest and update focused miner extraction tests.
5. Rewire crater/smudge call signatures and tests while preserving order.
6. Rewire combat ore-damage via either full context or deterministic reduction requests.
7. Remove or quarantine old `sim::miner::reduce_tiberium`.
8. Run focused tests after each stage.
9. Run final broader checks.

## Suggested Commands

Before Cargo:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
```

Focused tests:

```powershell
cargo test -q reduce_tiberium_partial_updates_overlay_data
cargo test -q reduce_tiberium_full_removal_overlaydata_11_returns_11
cargo test -q reduce_tiberium_full_removal_reseeds_spread_neighbors
cargo test -q cmin_overlaydata_11_extracts_11_bales
cargo test -q cmin_gem_overlaydata_11_extracts_11_gem_units
cargo test -q crater_reduce_tiberium_6_before_smudge_with_side_effects
cargo test -q combat_ore_damage_uses_shared_reduction
cargo test -q ore_growth_spread_queue_state_hashes
```

Final checks:

```powershell
cargo test -q sim::miner
cargo test -q sim::combat
cargo check -q
```

If module paths differ, use the bare test-name filters first.

## Non-Goals

- Do not change exact weapon AoE tiberium reduction gate or amount. That remains blocked on a dedicated RE pass.
- Do not implement the full natural YR growth/spread processor unless it becomes necessary for the depletion reseed API.
- Do not change CMIN return/refinery unload behavior except where upstream cargo amount changes naturally affect credits.
- Do not add render/UI dependencies to `sim/`.
- Do not preserve the old RA1 scan/reservoir behavior as a parity claim.
- Do not fix selected-unit tiberium pip formula in this patch; that formula remains separately blocked.

## Risks

- Borrowing full reduction context through combat may be awkward. If so, use deterministic reduction requests and apply them from `Simulation::advance_tick`.
- Overlay data semantics must be checked carefully at exact/full boundary. The required acceptance tests are the guard against reintroducing the 12-vs-11 bug.
- New queue state must be hashed and serialized, or replay/desync behavior will be wrong.
- Existing tests may assume `ResourceNode.remaining` is authoritative. Update them to use overlay-backed setup where the behavior is parity-sensitive.

## Done Criteria

- All active callers use `sim::tiberium::reduce_tiberium` or a request drained into it.
- No active combat/smudge code imports `sim::miner::reduce_tiberium`.
- Real overlay-backed `OverlayData=11` harvest returns 11 units for ore and gems.
- Partial reduction updates overlay data.
- Full removal clears overlay/resource state and emits dirty outputs.
- Full removal reseeds spread neighbors into persistent deterministic queue state.
- New queue state participates in state hashing.
- Focused tests and final `cargo check -q` pass, or any unrelated failures are identified.
