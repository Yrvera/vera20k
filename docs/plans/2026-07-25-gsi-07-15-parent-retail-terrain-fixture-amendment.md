# GSI-07.15 Parent Retail Terrain Fixture Amendment

Date: 2026-07-25  
Status: DRAFT FOR ADVERSARIAL REVIEW  
Parent:
`docs/plans/2026-07-24-gsi-07-15-level-zero-scan-archive-move-plan.md`  
Trigger:
post-prerequisite replay of
`production_stock_miners_filter_and_travel_to_present_zero_ring_one`

## Failure Evidence

After the approved outbound Drive prerequisite merged and the suspended parent
was replayed, the ring-0 parent oracle passed. The ring-1 production oracle
selected the intended present-zero cell but failed immediately after command
issue:

```text
assertion failed: entity.movement_target.is_some()
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4754 filtered out; finished in 0.14s
```

The new layered producer therefore returned `false` before installing a
`MovementTarget`.

## Ranked Diagnosis

1. **Fixture terrain profile is non-retail — established.**
   `clear_retail_cell` assigns `SpeedCostProfile::default()` to every cell and
   `staged_retail_terrain` changes only the ore cell's compatibility land type
   and `TerrainClass`. For Track, `TerrainCostGrid::from_resolved_terrain`
   receives no `[Tiberium]` speed entry, falls back to the compatibility
   passability matrix, and assigns cost `0` to Tiberium. Layered A* therefore
   rejects the selected target.
2. **Zone or occupancy rejection — contradicted by current evidence.**
   The test's live scan accepts the intended target through the same grid/zone
   authority, and no blocking entity occupies it.
3. **CMIN activation or NavCom rejection — contradicted by current evidence.**
   The test iterates HARV first, so the observed first failure does not depend
   on Teleport piggybacking. The seven prerequisite production tests also pass
   for both stock miners with retail terrain profiles.
4. **Present-zero production policy regression — contradicted by current
   evidence.** Ring 0 passes, and ring 1 selects the exact intended cell before
   command issue.

The older parent fixture remained accidentally usable only because the former
adjacent `issue_direct_move` path bypassed terrain/path authority. Once the
verified prerequisite routed outbound travel through layered Drive, the
fixture stopped representing retail input.

## Bounded Repair

Modify only the already parent-owned test file:

- `src/sim/miner/miner_tests.rs`

Do not change production Rust, the parent scan policy, Slave policy, the
prerequisite producer, pathfinding, terrain-cost logic, or any oracle
expectation.

### 1. Carry merged retail speed profiles in the fixture oracle

Extend `RetailMinerOracle`:

```rust
struct RetailMinerOracle {
    rules: RuleSet,
    overlays: OverlayTypeRegistry,
    config: MinerConfig,
    tib01: u8,
    clear_speed_costs: SpeedCostProfile,
    tiberium_speed_costs: SpeedCostProfile,
}
```

In `retail_miner_oracle`, after resolving `tib01`:

```rust
let clear_speed_costs = rules
    .terrain_rules
    .semantics_by_name("Clear")
    .expect("merged [Clear]")
    .speed_costs;
let tiberium_speed_costs = rules
    .terrain_rules
    .semantics_by_name("Tiberium")
    .expect("merged [Tiberium]")
    .speed_costs;
assert_eq!(clear_speed_costs.track, Some(100));
assert_eq!(tiberium_speed_costs.track, Some(70));
```

Return both profiles in `RetailMinerOracle`.

### 2. Construct resolved cells with the real profiles

Change the clear-cell helper to accept the merged Clear profile:

```rust
fn clear_retail_cell(
    rx: u16,
    ry: u16,
    speed_costs: SpeedCostProfile,
) -> ResolvedTerrainCell
```

Assign both:

```rust
speed_costs,
base_speed_costs: speed_costs,
```

Change the staged terrain helper to:

```rust
fn staged_retail_terrain(
    oracle: &RetailMinerOracle,
    cells: &[(u16, u16)],
) -> ResolvedTerrainGrid
```

Create all clear cells with `oracle.clear_speed_costs`. For every staged ore
cell, additionally assign:

```rust
cell.speed_costs = oracle.tiberium_speed_costs;
cell.allows_tiberium = true;
```

Retain the existing Tiberium land type, YR land type, terrain class, and
present-zero resource-node/overlay state.

### 3. Make world installation consume the oracle directly

Change:

```rust
fn install_retail_zero_world(
    sim: &mut Simulation,
    oracle: &RetailMinerOracle,
    grid: &PathGrid,
    nodes: &[((u16, u16), ResourceType)],
)
```

Call `staged_retail_terrain(oracle, ...)`, and use `oracle.tib01` for overlay
placement. Update exactly the three existing callers.

After building `sim.terrain_costs`, assert for every staged cell:

```rust
assert_eq!(
    sim.terrain_costs
        .get(&SpeedType::Track)
        .expect("Track terrain costs")
        .cost_at(cell.0, cell.1),
    70,
);
```

This prevents the parent oracle from silently falling back to a blocked
compatibility matrix again.

### 4. Strengthen the existing retail-zero assertion

In `assert_retail_zero`, after reading the resolved cell, require:

```rust
assert_eq!(terrain.speed_costs, oracle.tiberium_speed_costs);
assert!(terrain.allows_tiberium);
```

The test still asserts the node remains present with `remaining == 0` and the
TIB01 overlay remains at data/frame `0`; this amendment changes only the
resolved retail movement metadata used by production pathing.

## Validation

Format only:

```powershell
rustfmt --edition 2024 src/sim/miner/miner_tests.rs
```

Then run, serially under the global Cargo lease:

```powershell
cargo test production_stock_miners_filter_and_travel_to_present_zero_ring_one -- --nocapture
cargo test production_stock_miners_accept_present_zero_ring_zero -- --nocapture
cargo test production_full_harv_archives_zero_through_dock_and_drives_back -- --nocapture
cargo test standard_present_zero_scan_preserves_value_tie_and_first_ring_order -- --nocapture
cargo test slave_search_preserves_current_unverified_zero_rejection -- --nocapture
cargo test present_zero_resource_node_changes_state_hash -- --nocapture
```

If the critical ring-1 test still fails after the fixture proves Track cost
`70`, treat that result as a real later production divergence. Do not add a
bypass, restore direct movement, weaken the assertion, or change the
present-zero policy.

After these six pass, rerun all seven outbound-prerequisite production tests
and `cargo check -q`, then continue the parent commit/merge procedure while
retaining the recovery stash.

## Explicit Residual

The current Rust path grid also uses terrain percentages as A* route weights.
That pre-existing, separately documented parity drift is not the cause of this
failure: a retail Track cost of `70` is traversable, while the synthetic
profile's fallback cost `0` is blocked. This amendment does not claim or change
route-choice parity.

## Approval Question

Why should this be approved?

- It replaces synthetic `None` speed metadata with the exact merged retail
  `[Clear]` and `[Tiberium]` profiles already used by production.
- It makes the formerly hidden Track cost explicit (`70`) and executable.
- It changes no production behavior and weakens no oracle.
- It is the smallest correction needed for the parent oracle to exercise the
  new verified command authority rather than an impossible synthetic cell.

What evidence could still make it wrong?

- The merged retail loader could produce a Clear Track value other than `100`.
- The merged retail loader could produce a Track value other than `70`.
- Real resolved TIB01 cells could use different movement metadata than
  `[Tiberium]`.
- The ring-1 loop could remain red after cost `70`, proving another production
  divergence later than fixture construction.

Each condition is checked directly or remains a hard stop.
